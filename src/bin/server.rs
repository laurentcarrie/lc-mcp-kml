use std::collections::HashMap;
use std::io::BufWriter;
use std::sync::Arc;

use axum::{Router, Json, extract::{Path, State}, response::IntoResponse, http::StatusCode};
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};

use ::kml::types::KmlDocument;
use ::kml::{Kml, KmlReader, KmlWriter};
use lc_kml_utils::model::{InputData, EChoice};
use lc_kml_utils::processing::process_choices_with_resolver;

const S3_BUCKET: &str = "kml-laurent";
const S3_REGION: &str = "eu-west-3";
const S3_PREFIX: &str = "library/idf/";

struct AppState {
    s3_client: aws_sdk_s3::Client,
    llm_api_key: Option<String>,
}

async fn proxy_s3(Path(path): Path<String>) -> impl IntoResponse {
    let url = format!(
        "https://{}.s3.{}.amazonaws.com/{}{}",
        S3_BUCKET, S3_REGION, S3_PREFIX, path
    );
    match reqwest::get(&url).await {
        Ok(resp) if resp.status().is_success() => {
            let body = resp.bytes().await.unwrap_or_default();
            (StatusCode::OK, [
                ("content-type", "application/vnd.google-earth.kml+xml"),
                ("access-control-allow-origin", "*"),
            ], body).into_response()
        }
        Ok(resp) => (StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::NOT_FOUND), "Not found").into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, format!("S3 fetch error: {}", e)).into_response(),
    }
}

async fn list_s3(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut files = Vec::new();
    let mut continuation_token: Option<String> = None;

    loop {
        let mut req = state.s3_client.list_objects_v2().bucket(S3_BUCKET).prefix(S3_PREFIX);
        if let Some(token) = &continuation_token {
            req = req.continuation_token(token);
        }
        match req.send().await {
            Ok(output) => {
                for obj in output.contents() {
                    if let Some(key) = obj.key() {
                        if key.ends_with(".kml") {
                            let display_key = key.strip_prefix(S3_PREFIX).unwrap_or(key);
                            files.push(serde_json::json!({
                                "key": display_key,
                                "size": obj.size().unwrap_or(0),
                            }));
                        }
                    }
                }
                if output.is_truncated() == Some(true) {
                    continuation_token = output.next_continuation_token().map(|s| s.to_string());
                } else {
                    break;
                }
            }
            Err(e) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("S3 list error: {}", e)).into_response();
            }
        }
    }

    Json(files).into_response()
}

fn collect_kml_paths(choices: &[EChoice], paths: &mut std::collections::HashSet<String>) {
    for choice in choices {
        match choice {
            EChoice::ConcentricCircles(cc) => { paths.insert(cc.center.kml.clone()); }
            EChoice::Point(pd) => { paths.insert(pd.kml.clone()); }
            EChoice::UnionCircles(uc) => { for pd in &uc.centers { paths.insert(pd.kml.clone()); } }
            EChoice::Segments(seg) => { paths.insert(seg.kml.clone()); }
            EChoice::TriangleBisect(tb) => { paths.insert(tb.point1.kml.clone()); paths.insert(tb.point2.kml.clone()); }
            EChoice::RawKml(raw) => { paths.insert(raw.path.clone()); }
            EChoice::Folder(f) => { collect_kml_paths(&f.choices, paths); }
        }
    }
}

fn resolve_kml_from_cache<'a>(kml_cache: &'a mut HashMap<String, Kml>, path: &str) -> &'a Kml {
    if !kml_cache.contains_key(path) {
        panic!("KML not pre-fetched: {}", path);
    }
    kml_cache.get(path).unwrap()
}

#[derive(serde::Deserialize)]
struct PromptRequest {
    prompt: String,
}

async fn prompt_to_config(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PromptRequest>,
) -> impl IntoResponse {
    let api_key = match &state.llm_api_key {
        Some(k) => k.clone(),
        None => return (StatusCode::INTERNAL_SERVER_ERROR, "ANTHROPIC_API_KEY not set".to_string()).into_response(),
    };

    // Fetch library listing
    let mut library_files = Vec::new();
    let mut continuation_token: Option<String> = None;
    loop {
        let mut list_req = state.s3_client.list_objects_v2().bucket(S3_BUCKET).prefix(S3_PREFIX);
        if let Some(token) = &continuation_token {
            list_req = list_req.continuation_token(token);
        }
        match list_req.send().await {
            Ok(output) => {
                for obj in output.contents() {
                    if let Some(key) = obj.key() {
                        if key.ends_with(".kml") {
                            let display_key = key.strip_prefix(S3_PREFIX).unwrap_or(key);
                            library_files.push(display_key.to_string());
                        }
                    }
                }
                if output.is_truncated() == Some(true) {
                    continuation_token = output.next_continuation_token().map(|s| s.to_string());
                } else {
                    break;
                }
            }
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("S3 list error: {}", e)).into_response(),
        }
    }

    // For RER files, fetch placemark names so Claude knows the station names
    let mut kml_placemarks: HashMap<String, Vec<String>> = HashMap::new();
    for file in &library_files {
        if file.starts_with("rer/") || file.starts_with("bus/") {
            let url = format!(
                "https://{}.s3.{}.amazonaws.com/{}{}",
                S3_BUCKET, S3_REGION, S3_PREFIX, file
            );
            if let Ok(resp) = reqwest::get(&url).await {
                if resp.status().is_success() {
                    if let Ok(bytes) = resp.bytes().await {
                        if let Ok(kml) = KmlReader::from_reader(std::io::Cursor::new(bytes)).read() {
                            let names = collect_point_names(&kml);
                            if !names.is_empty() {
                                kml_placemarks.insert(file.clone(), names);
                            }
                        }
                    }
                }
            }
        }
    }

    let system_prompt = format!(
r#"You generate InputData JSON configurations for a KML map visualization tool.

The JSON schema is:
{{
  "choices": [EChoice, ...]
}}

EChoice is one of (externally tagged):
- {{"ConcentricCircles": {{"center": PointDefinition, "name": string, "v_radius": [float, ...], "circle_on_top": bool, "colors": [string, ...] or null}}}}
  Draws concentric circles around a point. colors are in AABBGGRR KML format (e.g. "ffff0000" = blue, "ff0000ff" = red, "ff00ff00" = green).
- {{"Point": PointDefinition}}
  Shows a single point on the map. When a color is set, points are displayed with a train/rail icon.
  To show ALL stations from a line, create one Point per placemark listed for that file.
- {{"Folder": {{"name": string, "choices": [EChoice, ...]}}}}
  Groups choices into a named folder.
- {{"UnionCircles": {{"name": string, "centers": [PointDefinition, ...], "radius": float, "circle_on_top": bool, "color": string or null}}}}
  Draws the union of circles around multiple points.
- {{"Segments": {{"name": string, "kml": string, "neighbours": [[string, string], ...], "color": string or null}}}}
  Draws line segments between pairs of named placemarks.
- {{"TriangleBisect": {{"point1": PointDefinition, "point2": PointDefinition, "radius_factor": float}}}}
  Draws a perpendicular bisector line between two points.
- {{"RawKml": {{"path": string, "color": string or null, "alpha": float}}}}
  Imports a raw KML file. Use for commune boundaries or other polygon files.

PointDefinition = {{"kml": string, "name": string, "color": string or null}}
  "kml" is a library path (e.g. "rer/RER-A.kml"), "name" is a placemark name within that file.

Colors are in KML AABBGGRR format: alpha, blue, green, red (hex). Examples:
- Blue: "ffff0000"
- Red: "ff0000ff"
- Green: "ff00ff00"
- Yellow: "ff00ffff"
- White: "ffffffff"

Available KML library files (use these paths in "kml" or "path" fields):
{}

Point placemarks available in key files:
{}

Commune boundary files are available under departement folders with naming pattern:
  <dept>/<INSEE_code>_<Commune-Name>.kml
Examples: "94/94017_Champigny-sur-Marne.kml", "94/94015_Bry-sur-Marne.kml", "75/75056_Paris.kml"
Departments available: 75 (Paris), 77 (Seine-et-Marne), 78 (Yvelines), 91 (Essonne), 92 (Hauts-de-Seine), 93 (Seine-Saint-Denis), 94 (Val-de-Marne), 95 (Val-d'Oise).
Use RawKml to display commune boundaries. You MUST use the correct INSEE code. Common communes:
  94017_Champigny-sur-Marne, 94015_Bry-sur-Marne, 94028_Creteil, 94033_Fontenay-sous-Bois,
  94042_Joinville-le-Pont, 94052_Maisons-Alfort, 94068_Saint-Mande, 94069_Saint-Maur-des-Fosses,
  94073_Sucy-en-Brie, 94076_Thiais, 94078_Villejuif, 94079_Villeneuve-le-Roi,
  94080_Villeneuve-Saint-Georges, 94081_Villiers-sur-Marne, 94071_Le-Perreux-sur-Marne,
  92012_Boulogne-Billancourt, 92044_Levallois-Perret, 92051_Neuilly-sur-Seine,
  93005_Aulnay-sous-Bois, 93008_Bobigny, 93048_Montreuil, 93053_Noisy-le-Grand,
  93066_Saint-Denis, 75056_Paris.
If unsure of an INSEE code for a commune not listed above, make your best guess based on the pattern.

When the user asks to color multiple communes such that adjacent/neighbouring ones have different colors, use your geographic knowledge of Île-de-France to determine which communes share a border, then apply a graph coloring (e.g. 3-4 colors) so that no two adjacent communes share the same color. Each RawKml entry can have its own color.

IMPORTANT: Return ONLY valid JSON, no markdown, no explanation. Just the InputData object."#,
        library_files.iter().map(|f| format!("  {}", f)).collect::<Vec<_>>().join("\n"),
        kml_placemarks.iter().map(|(file, names)| {
            format!("  {} → {}", file, names.join(", "))
        }).collect::<Vec<_>>().join("\n")
    );

    let body = serde_json::json!({
        "model": "claude-sonnet-4-20250514",
        "max_tokens": 16384,
        "system": system_prompt,
        "messages": [
            {"role": "user", "content": req.prompt}
        ]
    });

    let client = reqwest::Client::new();
    let resp: reqwest::Response = match client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return (StatusCode::BAD_GATEWAY, format!("Claude API error: {}", e)).into_response(),
    };

    let resp_status = resp.status().as_u16();
    let resp_body = resp.text().await.unwrap_or_default();
    if resp_status >= 400 {
        return (StatusCode::from_u16(resp_status).unwrap_or(StatusCode::BAD_GATEWAY), format!("Claude API {}: {}", resp_status, resp_body)).into_response();
    }

    let resp_json: serde_json::Value = match serde_json::from_str(&resp_body) {
        Ok(v) => v,
        Err(e) => return (StatusCode::BAD_GATEWAY, format!("Failed to parse Claude response: {}", e)).into_response(),
    };

    // Extract text from Claude response
    let text = resp_json["content"][0]["text"].as_str().unwrap_or("");

    // Try to parse as InputData to validate
    match serde_json::from_str::<InputData>(text) {
        Ok(input_data) => Json(input_data).into_response(),
        Err(e) => {
            // Try extracting JSON from markdown code block
            let cleaned = text.trim().trim_start_matches("```json").trim_start_matches("```").trim_end_matches("```").trim();
            match serde_json::from_str::<InputData>(cleaned) {
                Ok(input_data) => Json(input_data).into_response(),
                Err(_) => (StatusCode::BAD_REQUEST, format!("Claude returned invalid JSON: {}. Raw: {}", e, text)).into_response(),
            }
        }
    }
}

fn collect_point_names(kml: &Kml) -> Vec<String> {
    let mut names = Vec::new();
    collect_point_names_recursive(kml, &mut names);
    names
}

fn collect_point_names_recursive(kml: &Kml, names: &mut Vec<String>) {
    match kml {
        Kml::KmlDocument(doc) => doc.elements.iter().for_each(|e| collect_point_names_recursive(e, names)),
        Kml::Document { elements, .. } => elements.iter().for_each(|e| collect_point_names_recursive(e, names)),
        Kml::Folder(folder) => folder.elements.iter().for_each(|e| collect_point_names_recursive(e, names)),
        Kml::Placemark(p) => {
            if let Some(::kml::types::Geometry::Point(_)) = &p.geometry {
                if let Some(name) = &p.name {
                    names.push(name.clone());
                }
            }
        }
        _ => {}
    }
}

async fn generate_kml(Json(input_data): Json<InputData>) -> impl IntoResponse {
    eprintln!("generate_kml: {} choices, body: {}", input_data.choices.len(), serde_json::to_string(&input_data).unwrap_or_default());
    // Collect all referenced KML paths and fetch them from S3
    let mut paths = std::collections::HashSet::new();
    collect_kml_paths(&input_data.choices, &mut paths);

    let mut kml_cache: HashMap<String, Kml> = HashMap::new();
    for path in &paths {
        let url = format!(
            "https://{}.s3.{}.amazonaws.com/{}{}",
            S3_BUCKET, S3_REGION, S3_PREFIX, path
        );
        match reqwest::get(&url).await {
            Ok(resp) if resp.status().is_success() => {
                let bytes = resp.bytes().await.unwrap_or_default();
                match KmlReader::from_reader(std::io::Cursor::new(bytes)).read() {
                    Ok(kml) => { kml_cache.insert(path.clone(), kml); }
                    Err(e) => {
                        return (StatusCode::BAD_REQUEST, format!("Failed to parse KML '{}': {}", path, e)).into_response();
                    }
                }
            }
            Ok(_) => {
                return (StatusCode::BAD_REQUEST, format!("KML file not found in S3: {}", path)).into_response();
            }
            Err(e) => {
                return (StatusCode::BAD_GATEWAY, format!("Failed to fetch '{}': {}", path, e)).into_response();
            }
        }
    }

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        process_choices_with_resolver(&input_data.choices, &mut kml_cache, resolve_kml_from_cache)
    }));

    let output_elements = match result {
        Ok(elements) => elements,
        Err(e) => {
            let msg = e.downcast_ref::<String>().map(|s| s.as_str())
                .or_else(|| e.downcast_ref::<&str>().copied())
                .unwrap_or("Unknown error");
            return (StatusCode::BAD_REQUEST, format!("Processing error: {}", msg)).into_response();
        }
    };

    let output_kml_doc = Kml::KmlDocument(KmlDocument {
        elements: vec![Kml::Document {
            attrs: HashMap::new(),
            elements: output_elements,
        }],
        ..Default::default()
    });

    let mut buf = Vec::new();
    {
        let mut writer = KmlWriter::from_writer(BufWriter::new(&mut buf));
        if let Err(e) = writer.write(&output_kml_doc) {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("KML write error: {}", e)).into_response();
        }
    }

    (StatusCode::OK, [
        ("content-type", "application/vnd.google-earth.kml+xml"),
    ], buf).into_response()
}

#[tokio::main]
async fn main() {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let frontend_dir = std::env::var("FRONTEND_DIR").unwrap_or_else(|_| "frontend/dist".to_string());

    let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new(S3_REGION))
        .load()
        .await;
    let s3_client = aws_sdk_s3::Client::new(&config);
    let llm_api_key = std::env::var("ANTHROPIC_API_KEY").ok();
    if llm_api_key.is_none() {
        eprintln!("Warning: ANTHROPIC_API_KEY not set, /api/prompt will be disabled");
    }
    let state = Arc::new(AppState { s3_client, llm_api_key });

    let cors = CorsLayer::permissive();

    let app = Router::new()
        .route("/api/list", axum::routing::get(list_s3))
        .route("/api/prompt", axum::routing::post(prompt_to_config))
        .route("/api/generate", axum::routing::post(generate_kml))
        .route("/api/{*path}", axum::routing::get(proxy_s3))
        .fallback_service(
            ServeDir::new(&frontend_dir)
                .not_found_service(ServeFile::new(format!("{}/index.html", frontend_dir)))
        )
        .layer(cors)
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    println!("Serving frontend from {} and S3 proxy on {}", frontend_dir, addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
