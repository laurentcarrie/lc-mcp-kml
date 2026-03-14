use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{self, BufRead, BufWriter, Write};
use std::path::Path;

use ::kml::types::KmlDocument;
use ::kml::{Kml, KmlWriter};
use lc_kml_utils::model::{
    EChoice, Folder, InputData, PointDefinition, RawKml, RerLine, Segments, TriangleBisect,
    UnionCircles,
};
use lc_kml_utils::processing::process_choices;
use regex::Regex;
use serde::{Deserialize, Serialize};

const RER_LINES: [char; 5] = ['A', 'B', 'C', 'D', 'E'];

const RADII: [(u32, &str); 4] = [
    (500, "ff00ff00"),  // green
    (1000, "ffff0000"), // blue
    (1500, "ffffff00"), // cyan
    (2000, "ff0000ff"), // red
];

const LINE_COLORS: [(&str, &str); 5] = [
    ("A", "ff0000ff"), // red
    ("B", "ffff0000"), // blue
    ("C", "ff00ffff"), // yellow
    ("D", "ff00ff00"), // green
    ("E", "ffff00ff"), // magenta
];

const USER_INPUT_FILE: &str = "RER-user-input.yml";

#[derive(Deserialize, Serialize, Default)]
struct RerUserInput {
    lines: String,
    radii: String,
    show_bisect: String,
    bisect_radius_factor: f64,
    commune_filter: Vec<String>,
    commune_alpha: f64,
}

fn line_color(letter: char) -> &'static str {
    LINE_COLORS
        .iter()
        .find(|(l, _)| *l == letter.to_string())
        .map(|(_, c)| *c)
        .unwrap_or("ffffffff")
}

fn read_stations(kml_file: &str) -> Vec<String> {
    let content = std::fs::read_to_string(kml_file).expect(&format!("Failed to read {}", kml_file));
    let re = Regex::new(r"<Placemark>.*?<name>(.*?)</name>").unwrap();
    re.captures_iter(&content)
        .map(|c| c[1].to_string())
        .collect()
}

fn prompt(msg: &str) -> String {
    print!("{}", msg);
    io::stdout().flush().unwrap();
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line).unwrap();
    line.trim().to_string()
}

/// Prompt with a default value shown. Returns user input or default if empty.
fn prompt_with_default(msg: &str, default: &str) -> String {
    if default.is_empty() {
        prompt(msg)
    } else {
        let input = prompt(&format!("{} [{}]: ", msg, default));
        if input.is_empty() {
            default.to_string()
        } else {
            input
        }
    }
}

/// Simple fuzzy match: all characters of the query appear in order in the target (case-insensitive).
fn fuzzy_matches(target: &str, query: &str) -> bool {
    let target_lower = target.to_lowercase();
    let query_lower = query.to_lowercase();
    let mut target_chars = target_lower.chars();
    for qc in query_lower.chars() {
        if target_chars.find(|&tc| tc == qc).is_none() {
            return false;
        }
    }
    true
}

/// Extract commune name from a KML filename like "75056_Paris.kml" -> "Paris"
fn commune_name_from_filename(filename: &str) -> String {
    let stem = filename.strip_suffix(".kml").unwrap_or(filename);
    match stem.find('_') {
        Some(pos) => stem[pos + 1..].replace('_', " "),
        None => stem.to_string(),
    }
}

/// Extract department code (first 2 digits) from a KML filename like "75056_Paris.kml" -> "75"
fn dept_code_from_filename(filename: &str) -> String {
    let stem = filename.strip_suffix(".kml").unwrap_or(filename);
    stem.chars().take(2).collect()
}

// 4 colors for the map coloring (ABGR format for KML)
const MAP_COLORS: [&str; 5] = [
    "ff0000ff", // red
    "ffff8800", // orange
    "ff00cc00", // green
    "ffcc00cc", // magenta
    "ff00ccff", // yellow
];

/// Extract edges (pairs of consecutive coordinates) from a KML file.
/// Returns normalized edge keys for adjacency detection.
fn extract_edges_from_kml(path: &str) -> HashSet<String> {
    let content = std::fs::read_to_string(path).unwrap_or_default();
    let re = Regex::new(r"<coordinates>([\s\S]*?)</coordinates>").unwrap();
    let mut edges = HashSet::new();

    for cap in re.captures_iter(&content) {
        let coords_str = &cap[1];
        let coords: Vec<&str> = coords_str
            .split_whitespace()
            .filter(|s| !s.is_empty())
            .collect();

        for pair in coords.windows(2) {
            // Normalize edge: sort the two endpoints so A-B == B-A
            let (a, b) = if pair[0] <= pair[1] {
                (pair[0], pair[1])
            } else {
                (pair[1], pair[0])
            };
            edges.insert(format!("{}|{}", a, b));
        }
    }
    edges
}

/// DSatur graph coloring algorithm.
/// Returns a color assignment (index into MAP_COLORS) for each node.
fn dsatur_coloring(adjacency: &[Vec<usize>]) -> Vec<usize> {
    let n = adjacency.len();
    let mut color: Vec<Option<usize>> = vec![None; n];
    let mut saturation: Vec<HashSet<usize>> = vec![HashSet::new(); n];

    for _ in 0..n {
        // Pick uncolored vertex with highest saturation, break ties by degree
        let next = (0..n)
            .filter(|&i| color[i].is_none())
            .max_by_key(|&i| (saturation[i].len(), adjacency[i].len()))
            .unwrap();

        // Find smallest color not used by neighbors
        let used: HashSet<usize> = adjacency[next]
            .iter()
            .filter_map(|&nb| color[nb])
            .collect();
        let c = (0..4).find(|c| !used.contains(c)).unwrap_or_else(|| {
            // Fallback beyond 4 if needed (shouldn't happen for planar graphs)
            (0..).find(|c| !used.contains(c)).unwrap()
        });

        color[next] = Some(c);

        // Update saturation of uncolored neighbors
        for &nb in &adjacency[next] {
            if color[nb].is_none() {
                saturation[nb].insert(c);
            }
        }
    }

    color.into_iter().map(|c| c.unwrap()).collect()
}

/// Compute 4-coloring for a list of commune KML paths.
/// Returns a map from path to color string (ABGR).
fn four_color_communes(paths: &[String]) -> HashMap<String, String> {
    println!("Computing 4-coloring for {} communes...", paths.len());

    // Extract edges for each commune
    let edge_sets: Vec<HashSet<String>> = paths
        .iter()
        .map(|p| extract_edges_from_kml(p))
        .collect();

    // Build adjacency graph: two communes are adjacent if they share at least one edge
    let mut adjacency: Vec<Vec<usize>> = vec![vec![]; paths.len()];

    // Build an edge -> commune index map
    let mut edge_to_communes: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, edges) in edge_sets.iter().enumerate() {
        for edge in edges {
            edge_to_communes.entry(edge.clone()).or_default().push(i);
        }
    }

    // Communes sharing an edge are adjacent
    let mut adj_set: Vec<HashSet<usize>> = vec![HashSet::new(); paths.len()];
    for communes in edge_to_communes.values() {
        for &a in communes {
            for &b in communes {
                if a != b {
                    adj_set[a].insert(b);
                    adj_set[b].insert(a);
                }
            }
        }
    }
    for (i, s) in adj_set.into_iter().enumerate() {
        adjacency[i] = s.into_iter().collect();
    }

    // Run DSatur
    let coloring = dsatur_coloring(&adjacency);

    let max_color = coloring.iter().copied().max().unwrap_or(0);
    println!(
        "4-coloring done: used {} colors",
        max_color + 1
    );

    paths
        .iter()
        .zip(coloring.iter())
        .map(|(path, &c)| (path.clone(), MAP_COLORS[c % MAP_COLORS.len()].to_string()))
        .collect()
}

/// Read all placemark points from a KML file, returning (name, lon, lat).
fn read_points_from_kml(kml_file: &str) -> Vec<(String, f64, f64)> {
    let content = std::fs::read_to_string(kml_file).unwrap_or_default();
    let re = Regex::new(
        r"(?s)<Placemark[^>]*>.*?<name>(.*?)</name>.*?<Point>.*?<coordinates>([\d.,\-\s]+)</coordinates>.*?</Point>"
    ).unwrap();
    let mut points = Vec::new();
    for cap in re.captures_iter(&content) {
        let name = cap[1].trim().to_string();
        let coords_str = cap[2].trim();
        let parts: Vec<&str> = coords_str.split(',').collect();
        if parts.len() >= 2 {
            if let (Ok(lon), Ok(lat)) = (parts[0].trim().parse::<f64>(), parts[1].trim().parse::<f64>()) {
                points.push((name, lon, lat));
            }
        }
    }
    points
}

/// Read station coordinates from a RER KML file, returning (name, lon, lat).
fn read_station_coords(kml_file: &str) -> Vec<(String, f64, f64)> {
    read_points_from_kml(kml_file)
}

/// Haversine distance in meters between two (lon, lat) points.
fn haversine_distance(lon1: f64, lat1: f64, lon2: f64, lat2: f64) -> f64 {
    let earth_radius = 6_371_000.0;
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (dlon / 2.0).sin().powi(2);
    2.0 * earth_radius * a.sqrt().asin()
}

/// Query OSRM for a walking route between two points. Returns the route coordinates.
fn ors_walking_route(api_key: &str, from_lon: f64, from_lat: f64, to_lon: f64, to_lat: f64) -> Option<(Vec<(f64, f64)>, f64)> {
    let url = format!(
        "https://api.openrouteservice.org/v2/directions/foot-walking?api_key={}&start={},{}&end={},{}",
        api_key, from_lon, from_lat, to_lon, to_lat
    );
    let output = match std::process::Command::new("curl")
        .args(["-sk", &url])
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            eprintln!("    curl failed: {}", e);
            return None;
        }
    };
    if !output.status.success() {
        eprintln!("    curl returned non-zero exit code");
        return None;
    }
    let body: serde_json::Value = match serde_json::from_slice(&output.stdout) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("    ORS JSON parse failed: {}", e);
            return None;
        }
    };

    let feature = body.get("features")?.get(0)?;
    let distance = feature.get("properties")?.get("segments")?.get(0)?.get("distance")?.as_f64()?;
    let coords = feature.get("geometry")?.get("coordinates")?.as_array()?;
    let points: Vec<(f64, f64)> = coords
        .iter()
        .filter_map(|c: &serde_json::Value| {
            let arr = c.as_array()?;
            Some((arr.get(0)?.as_f64()?, arr.get(1)?.as_f64()?))
        })
        .collect();
    Some((points, distance))
}

fn load_api_key() -> Option<String> {
    std::fs::read_to_string("private.key").ok().map(|s| s.trim().to_string())
}

fn load_user_input() -> RerUserInput {
    if Path::new(USER_INPUT_FILE).exists() {
        let file = File::open(USER_INPUT_FILE).expect("Failed to open user input file");
        serde_yaml::from_reader(file).unwrap_or_default()
    } else {
        RerUserInput::default()
    }
}

fn save_user_input(input: &RerUserInput) {
    let file = File::create(USER_INPUT_FILE).expect("Failed to create user input file");
    serde_yaml::to_writer(file, input).expect("Failed to write user input file");
    println!("User input saved to {}", USER_INPUT_FILE);
}

fn main() {
    let prev = load_user_input();

    // Ask which RER lines
    println!("Available RER lines: A B C D E");
    let lines_input = prompt_with_default("Which lines? (e.g. A,B or * for all)", &prev.lines);
    let selected_lines: Vec<char> = if lines_input == "*" {
        RER_LINES.to_vec()
    } else {
        lines_input
            .split(',')
            .map(|s| s.trim().to_uppercase().chars().next().unwrap())
            .filter(|c| RER_LINES.contains(c))
            .collect()
    };
    if selected_lines.is_empty() {
        eprintln!("No valid lines selected.");
        std::process::exit(1);
    }

    // Ask which radii
    println!("Available radii: 500 1000 1500 2000");
    let radii_input = prompt_with_default("Which radii? (e.g. 500,1000 or * for all)", &prev.radii);
    let selected_radii: Vec<u32> = if radii_input == "*" {
        RADII.iter().map(|(r, _)| *r).collect()
    } else {
        radii_input
            .split(',')
            .filter_map(|s| s.trim().parse::<u32>().ok())
            .filter(|r| RADII.iter().any(|(rr, _)| rr == r))
            .collect()
    };

    // Ask about bisect triangles
    let bisect_default = if prev.show_bisect.is_empty() {
        "y".to_string()
    } else {
        prev.show_bisect.clone()
    };
    let bisect_input = prompt_with_default("Display bisect triangles? (y/n)", &bisect_default);
    let show_bisect = bisect_input.starts_with('y') || bisect_input.starts_with('Y');

    let factor_default = if prev.bisect_radius_factor > 0.0 {
        format!("{}", prev.bisect_radius_factor)
    } else {
        "0.6".to_string()
    };
    let bisect_radius_factor = if show_bisect {
        let input = prompt_with_default("Radius factor", &factor_default);
        input.parse::<f64>().unwrap_or(0.6)
    } else {
        factor_default.parse::<f64>().unwrap_or(0.6)
    };

    // Ask about commune filters
    let commune_filters: Vec<String> = if Path::new("idf").exists() {
        let prev_display = prev.commune_filter.join(",");
        let input = prompt_with_default(
            "Commune filters (comma-separated fuzzy, empty to skip)",
            &prev_display,
        );
        if input.is_empty() {
            vec![]
        } else {
            input.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
        }
    } else {
        vec![]
    };

    let commune_alpha = if !commune_filters.is_empty() {
        let alpha_default = if prev.commune_alpha > 0.0 {
            format!("{}", prev.commune_alpha)
        } else {
            "0.1".to_string()
        };
        let input = prompt_with_default("Commune fill alpha (0.0-1.0)", &alpha_default);
        input.parse::<f64>().unwrap_or(0.1).clamp(0.0, 1.0)
    } else {
        prev.commune_alpha
    };

    // Save user input
    let user_input = RerUserInput {
        lines: lines_input,
        radii: radii_input,
        show_bisect: bisect_input,
        bisect_radius_factor,
        commune_filter: commune_filters.clone(),
        commune_alpha,
    };
    save_user_input(&user_input);

    println!(
        "\nGenerating for lines: {:?}, radii: {:?}, bisect: {}",
        selected_lines, selected_radii, show_bisect
    );

    // Build InputData
    let mut choices: Vec<EChoice> = Vec::new();

    for &letter in &selected_lines {
        let kml_file = format!("RER-{}.kml", letter);
        let yml_file = format!("RER-{}.yml", letter);
        let color = line_color(letter);
        let stations = read_stations(&kml_file);

        let mut line_choices: Vec<EChoice> = Vec::new();

        // Gares folder
        let gares: Vec<EChoice> = stations
            .iter()
            .map(|name| {
                EChoice::Point(PointDefinition {
                    kml: kml_file.clone(),
                    name: name.clone(),
                    color: Some(color.to_string()),
                    ..Default::default()
                })
            })
            .collect();
        line_choices.push(EChoice::Folder(Folder {
            name: "Gares".to_string(),
            choices: gares,
        }));

        // Union circles per selected radius
        for &radius in &selected_radii {
            let (_, radius_color) = RADII.iter().find(|(r, _)| *r == radius).unwrap();
            let centers: Vec<PointDefinition> = stations
                .iter()
                .map(|name| PointDefinition {
                    kml: kml_file.clone(),
                    name: name.clone(),
                    color: None,
                    ..Default::default()
                })
                .collect();
            line_choices.push(EChoice::Folder(Folder {
                name: format!("{}m", radius),
                choices: vec![EChoice::UnionCircles(UnionCircles {
                    name: format!("RER-{}-{}m", letter, radius),
                    centers,
                    radius: radius as f64,
                    circle_on_top: true,
                    color: Some(radius_color.to_string()),
                })],
            }));
        }

        // Segments and bisect triangles from YML
        if Path::new(&yml_file).exists() {
            let yml = File::open(&yml_file).expect(&format!("Failed to open {}", yml_file));
            let rer_line: RerLine =
                serde_yaml::from_reader(yml).expect(&format!("Failed to parse {}", yml_file));

            line_choices.push(EChoice::Folder(Folder {
                name: "Lignes".to_string(),
                choices: vec![EChoice::Segments(Segments {
                    name: format!("RER-{}", letter),
                    kml: kml_file.clone(),
                    neighbours: rer_line.neighbours.clone(),
                    color: Some(color.to_string()),
                })],
            }));

            if show_bisect {
                let bisects: Vec<EChoice> = rer_line
                    .neighbours
                    .iter()
                    .map(|pair| {
                        EChoice::TriangleBisect(TriangleBisect {
                            point1: PointDefinition {
                                kml: kml_file.clone(),
                                name: pair[0].clone(),
                                color: None,
                                ..Default::default()
                            },
                            point2: PointDefinition {
                                kml: kml_file.clone(),
                                name: pair[1].clone(),
                                color: None,
                                ..Default::default()
                            },
                            radius_factor: bisect_radius_factor,
                        })
                    })
                    .collect();
                line_choices.push(EChoice::Folder(Folder {
                    name: "Bisects".to_string(),
                    choices: bisects,
                }));
            }
        }

        choices.push(EChoice::Folder(Folder {
            name: format!("RER-{}", letter),
            choices: line_choices,
        }));
    }

    // Load IDF communes (fuzzy filtered, grouped by dept code, sorted alphabetically)
    let idf_dir = Path::new("idf");
    if idf_dir.exists() && !commune_filters.is_empty() {
        // Collect all matching communes: (dept_code, commune_name, path)
        let mut matched: Vec<(String, String, String)> = Vec::new();

        let mut dept_dirs: Vec<_> = std::fs::read_dir(idf_dir)
            .expect("Failed to read idf directory")
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .collect();
        dept_dirs.sort_by_key(|e| e.file_name());

        for dept_entry in dept_dirs {
            let mut kml_files: Vec<_> = std::fs::read_dir(dept_entry.path())
                .expect("Failed to read department directory")
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext == "kml")
                        .unwrap_or(false)
                })
                .collect();
            kml_files.sort_by_key(|e| e.file_name());

            for kml_entry in kml_files {
                let filename = kml_entry.file_name().to_string_lossy().to_string();
                let name = commune_name_from_filename(&filename);
                let match_all = commune_filters.iter().any(|f| f == "*");
                if match_all || commune_filters.iter().any(|f| fuzzy_matches(&name, f)) {
                    let dept = dept_code_from_filename(&filename);
                    let path = kml_entry.path().to_string_lossy().to_string();
                    println!("  + {} ({})", name, dept);
                    matched.push((dept, name, path));
                }
            }
        }

        println!("{} communes matched", matched.len());

        // Compute 4-coloring across all matched communes
        let all_paths: Vec<String> = matched.iter().map(|(_, _, p)| p.clone()).collect();
        let color_map = four_color_communes(&all_paths);

        // Group by department code, sort communes alphabetically within each
        let mut by_dept: HashMap<String, Vec<(String, String)>> = HashMap::new();
        for (dept, name, path) in matched {
            by_dept.entry(dept).or_default().push((name, path));
        }

        let mut dept_codes: Vec<String> = by_dept.keys().cloned().collect();
        dept_codes.sort();

        let mut dept_folders: Vec<EChoice> = Vec::new();
        for dept in dept_codes {
            let mut communes = by_dept.remove(&dept).unwrap();
            communes.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
            let commune_choices: Vec<EChoice> = communes
                .into_iter()
                .map(|(_, path)| {
                    let color = color_map.get(&path).cloned();
                    EChoice::RawKml(RawKml { path, color, alpha: commune_alpha })
                })
                .collect();
            dept_folders.push(EChoice::Folder(Folder {
                name: format!("Dept {}", dept),
                choices: commune_choices,
            }));
        }

        if !dept_folders.is_empty() {
            choices.push(EChoice::Folder(Folder {
                name: "Communes IDF".to_string(),
                choices: dept_folders,
            }));
        }
    }

    let input_data = InputData { choices, error: None };

    // Write debug YAML
    let debug_yml = "rer-debug-input.yml";
    let yml_file = File::create(debug_yml).expect("Failed to create debug YAML");
    serde_yaml::to_writer(yml_file, &input_data).expect("Failed to write debug YAML");
    println!("Debug YAML written to {}", debug_yml);

    // Generate KML directly using processing module
    let output_kml = "rer-out.kml";
    let mut kml_cache: HashMap<String, Kml> = HashMap::new();
    let mut output_elements = process_choices(&input_data.choices, &mut kml_cache);

    // Walking routes from local.kml points to closest loaded stations
    if Path::new("local.kml").exists() {
        use ::kml::types::{Coord, LineString as KmlLineString, Placemark, Geometry};

        let api_key = load_api_key().unwrap_or_else(|| {
            eprintln!("Warning: private.key not found, walking routes disabled");
            String::new()
        });
        let local_points = read_points_from_kml("local.kml");
        if !local_points.is_empty() && !api_key.is_empty() {
            // Collect all station coordinates from loaded RER lines
            let mut all_stations: Vec<(String, f64, f64)> = Vec::new();
            for &letter in &selected_lines {
                let kml_file = format!("RER-{}.kml", letter);
                let coords = read_station_coords(&kml_file);
                for (name, lon, lat) in coords {
                    all_stations.push((format!("{} (RER-{})", name, letter), lon, lat));
                }
            }

            if !all_stations.is_empty() {
                // Route style
                let style_id = "walking_route_style";
                output_elements.push(Kml::Style(::kml::types::Style {
                    id: Some(style_id.to_string()),
                    line: Some(::kml::types::LineStyle {
                        color: "ff00aaff".to_string(),
                        width: 4.0,
                        ..Default::default()
                    }),
                    ..Default::default()
                }));

                let mut route_elements: Vec<Kml> = Vec::new();

                for (point_name, plon, plat) in &local_points {
                    // Sort stations by distance
                    let mut sorted_stations: Vec<_> = all_stations
                        .iter()
                        .map(|(name, lon, lat)| {
                            let d = haversine_distance(*plon, *plat, *lon, *lat);
                            (name, *lon, *lat, d)
                        })
                        .collect();
                    sorted_stations.sort_by(|a, b| a.3.partial_cmp(&b.3).unwrap());

                    // Try closest station; if ratio > 1.5, also try next 4 and pick shortest walk
                    let mut best: Option<(String, f64, Vec<(f64, f64)>)> = None;
                    let mut first_ratio_ok = false;
                    for (i, (sname, slon, slat, straight)) in sorted_stations.iter().take(5).enumerate() {
                        // If first station had ratio <= 1.5, no need to try more
                        if i > 0 && first_ratio_ok {
                            break;
                        }
                        println!(
                            "  {} -> {} ({:.0}m straight line){}",
                            point_name, sname, straight,
                            if i > 0 { " [candidate]" } else { "" }
                        );

                        match ors_walking_route(&api_key, *plon, *plat, *slon, *slat) {
                            Some((coords, walk_dist)) => {
                                let ratio = walk_dist / straight;
                                println!(
                                    "    Walking route: {:.0}m, {} points (ratio: {:.2})",
                                    walk_dist, coords.len(), ratio
                                );
                                if i == 0 && ratio <= 1.5 {
                                    first_ratio_ok = true;
                                }
                                if best.is_none() || walk_dist < best.as_ref().unwrap().1 {
                                    best = Some((sname.to_string(), walk_dist, coords));
                                }
                            }
                            None => {
                                eprintln!("    Failed to get walking route for {}", point_name);
                            }
                        }
                    }
                    if let Some((sname, walk_dist, coords)) = best {
                        println!("  => Picked: {} -> {} ({:.0}m walk)", point_name, sname, walk_dist);
                        let line_string = KmlLineString {
                            coords: coords
                                .iter()
                                .map(|(lon, lat)| Coord { x: *lon, y: *lat, z: Some(0.0) })
                                .collect(),
                            ..Default::default()
                        };
                        route_elements.push(Kml::Placemark(Placemark {
                            name: Some(format!(
                                "{} \u{2192} {} ({:.0}m)",
                                point_name, sname, walk_dist
                            )),
                            style_url: Some(format!("#{}", style_id)),
                            geometry: Some(Geometry::LineString(line_string)),
                            ..Default::default()
                        }));
                    } else {
                        eprintln!("  No suitable walking route found for {}", point_name);
                    }
                }

                if !route_elements.is_empty() {
                    output_elements.push(Kml::Folder(::kml::types::Folder {
                        name: Some("Trajets piétons".to_string()),
                        elements: route_elements,
                        ..Default::default()
                    }));
                }
            }
        }
    }

    let mut kml_attrs = HashMap::new();
    kml_attrs.insert(
        "xmlns".to_string(),
        "http://www.opengis.net/kml/2.2".to_string(),
    );
    let output_kml_doc = Kml::KmlDocument(KmlDocument {
        attrs: kml_attrs,
        elements: vec![Kml::Document {
            attrs: HashMap::new(),
            elements: output_elements,
        }],
        ..Default::default()
    });

    let file = File::create(output_kml).expect("Failed to create output KML file");
    let mut buf_writer = BufWriter::new(file);
    buf_writer
        .write_all(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n")
        .expect("Failed to write XML declaration");
    let mut writer = KmlWriter::from_writer(buf_writer);
    writer.write(&output_kml_doc).expect("Failed to write KML");
    println!("Written to {}", output_kml);
}
