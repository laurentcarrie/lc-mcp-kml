use std::collections::HashMap;
use std::io::BufWriter;

use ::kml::types::KmlDocument;
use ::kml::{Kml, KmlReader, KmlWriter};
use anyhow::Result;
use rmcp::{
    ServerHandler,
    ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
};
use serde::Deserialize;

use lc_kml_utils::model::{InputData, EChoice};
use lc_kml_utils::processing::process_choices_with_resolver;

static S3_BUCKET: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    std::env::var("S3_BUCKET").expect("S3_BUCKET environment variable must be set")
});
static S3_REGION: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    std::env::var("S3_REGION").unwrap_or_else(|_| "eu-west-3".to_string())
});
const S3_PREFIX: &str = "library/idf/";

fn s3_url(path: &str) -> String {
    format!(
        "https://{}.s3.{}.amazonaws.com/{}{}",
        *S3_BUCKET, *S3_REGION, S3_PREFIX, path
    )
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListPlacemarksParams {
    /// KML library path, e.g. "rer/RER-A.kml"
    pub kml_path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GenerateKmlParams {
    /// InputData configuration as a JSON string
    pub config_json: String,
}

#[derive(Debug, Clone)]
pub struct KmlMcpServer {
    s3_client: aws_sdk_s3::Client,
    tool_router: ToolRouter<Self>,
}

impl KmlMcpServer {
    pub fn new(s3_client: aws_sdk_s3::Client) -> Self {
        Self {
            s3_client,
            tool_router: Self::tool_router(),
        }
    }
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
            EChoice::Route(rt) => { if !rt.from.kml.is_empty() { paths.insert(rt.from.kml.clone()); } if !rt.to.kml.is_empty() { paths.insert(rt.to.kml.clone()); } }
            EChoice::Folder(f) => { collect_kml_paths(&f.choices, paths); }
            EChoice::BulkRawKml(_) => {}
        }
    }
}

fn resolve_kml_from_cache<'a>(kml_cache: &'a mut HashMap<String, Kml>, path: &str) -> &'a Kml {
    if !kml_cache.contains_key(path) {
        panic!("KML not pre-fetched: {}", path);
    }
    kml_cache.get(path).unwrap()
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

async fn fetch_kml_from_s3(path: &str) -> std::result::Result<Kml, String> {
    let url = s3_url(path);
    let resp = reqwest::get(&url).await.map_err(|e| format!("Failed to fetch '{}': {}", path, e))?;
    if !resp.status().is_success() {
        return Err(format!("KML file not found in S3: {}", path));
    }
    let bytes = resp.bytes().await.map_err(|e| format!("Failed to read '{}': {}", path, e))?;
    KmlReader::from_reader(std::io::Cursor::new(bytes))
        .read()
        .map_err(|e| format!("Failed to parse KML '{}': {}", path, e))
}

#[tool_router]
impl KmlMcpServer {
    #[tool(description = "List all KML files available in the S3 library. Returns file paths organized by folder (rer/, bus/, communes/, etc). Use these paths as kml_path in other tools.")]
    async fn list_library(&self) -> Result<String, String> {
        let mut files = Vec::new();
        let mut continuation_token: Option<String> = None;

        loop {
            let mut req = self.s3_client.list_objects_v2().bucket(S3_BUCKET.as_str()).prefix(S3_PREFIX);
            if let Some(token) = &continuation_token {
                req = req.continuation_token(token);
            }
            let output = req.send().await.map_err(|e| format!("S3 list error: {}", e))?;
            for obj in output.contents() {
                if let Some(key) = obj.key() {
                    if key.ends_with(".kml") {
                        let display_key = key.strip_prefix(S3_PREFIX).unwrap_or(key);
                        files.push(display_key.to_string());
                    }
                }
            }
            if output.is_truncated() == Some(true) {
                continuation_token = output.next_continuation_token().map(|s| s.to_string());
            } else {
                break;
            }
        }

        serde_json::to_string_pretty(&files).map_err(|e| e.to_string())
    }

    #[tool(description = "List all point placemark names in a KML file. Use a path from list_library (e.g. 'rer/RER-A.kml'). Returns station/point names that can be used in generate_kml configurations.")]
    async fn list_placemarks(
        &self,
        Parameters(params): Parameters<ListPlacemarksParams>,
    ) -> Result<String, String> {
        let kml = fetch_kml_from_s3(&params.kml_path).await?;
        let names = collect_point_names(&kml);
        serde_json::to_string_pretty(&names).map_err(|e| e.to_string())
    }

    #[tool(description = r#"Generate a KML file from an InputData configuration. The config_json parameter must be a JSON object with this schema:
{
  "choices": [EChoice, ...]
}

EChoice is one of (externally tagged):
- {"ConcentricCircles": {"center": PointDefinition, "name": string, "v_radius": [float, ...], "circle_on_top": bool, "colors": [string, ...] or null}}
- {"Point": PointDefinition}
- {"Folder": {"name": string, "choices": [EChoice, ...]}}
- {"UnionCircles": {"name": string, "centers": [PointDefinition, ...], "radius": float, "circle_on_top": bool, "color": string or null}}
- {"Segments": {"name": string, "kml": string, "neighbours": [[string, string], ...], "color": string or null}}
- {"TriangleBisect": {"point1": PointDefinition, "point2": PointDefinition, "radius_factor": float}}
- {"RawKml": {"path": string, "color": string or null, "alpha": float}}

PointDefinition = {"kml": string, "name": string, "color": string or null}
"kml" is a library path (e.g. "rer/RER-A.kml"), "name" is a placemark name within that file.

Colors are in KML AABBGGRR format (hex): alpha, blue, green, red.
Examples: blue="ffff0000", red="ff0000ff", green="ff00ff00"

Returns KML XML that can be saved as a .kml file and opened in Google Earth."#)]
    async fn generate_kml(
        &self,
        Parameters(params): Parameters<GenerateKmlParams>,
    ) -> Result<String, String> {
        let input_data: InputData = serde_json::from_str(&params.config_json)
            .map_err(|e| format!("Invalid config JSON: {}", e))?;

        // Collect and pre-fetch all referenced KML files
        let mut paths = std::collections::HashSet::new();
        collect_kml_paths(&input_data.choices, &mut paths);

        let mut kml_cache: HashMap<String, Kml> = HashMap::new();
        for path in &paths {
            let kml = fetch_kml_from_s3(path).await?;
            kml_cache.insert(path.clone(), kml);
        }

        // Process choices
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            process_choices_with_resolver(&input_data.choices, &mut kml_cache, resolve_kml_from_cache)
        }));

        let output_elements = match result {
            Ok(elements) => elements,
            Err(e) => {
                let msg = e.downcast_ref::<String>().map(|s| s.as_str())
                    .or_else(|| e.downcast_ref::<&str>().copied())
                    .unwrap_or("Unknown error");
                return Err(format!("Processing error: {}", msg));
            }
        };

        // Write KML output
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
            writer.write(&output_kml_doc)
                .map_err(|e| format!("KML write error: {}", e))?;
        }

        String::from_utf8(buf).map_err(|e| format!("UTF-8 error: {}", e))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for KmlMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder().enable_tools().build()
        ).with_instructions(
            "KML map visualization tool. Use list_library to discover available KML files, \
             list_placemarks to find station/point names, and generate_kml to create KML \
             visualizations with circles, unions, segments, and more."
        )
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new(S3_REGION.clone()))
        .load()
        .await;
    let s3_client = aws_sdk_s3::Client::new(&config);

    let server = KmlMcpServer::new(s3_client);
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
