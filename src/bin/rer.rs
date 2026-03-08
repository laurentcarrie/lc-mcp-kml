use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufWriter, Write};
use std::path::Path;

use ::kml::types::KmlDocument;
use ::kml::{Kml, KmlWriter};
use lc_kml_utils::model::*;
use lc_kml_utils::processing::process_choices;
use regex::Regex;

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

fn main() {
    // Ask which RER lines
    println!("Available RER lines: A B C D E");
    let input = prompt("Which lines? (e.g. A,B or * for all): ");
    let selected_lines: Vec<char> = if input == "*" {
        RER_LINES.to_vec()
    } else {
        input
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
    let input = prompt("Which radii? (e.g. 500,1000 or * for all): ");
    let selected_radii: Vec<u32> = if input == "*" {
        RADII.iter().map(|(r, _)| *r).collect()
    } else {
        input
            .split(',')
            .filter_map(|s| s.trim().parse::<u32>().ok())
            .filter(|r| RADII.iter().any(|(rr, _)| rr == r))
            .collect()
    };

    // Ask about bisect triangles
    let input = prompt("Display bisect triangles? (y/n): ");
    let show_bisect = input.starts_with('y') || input.starts_with('Y');

    let bisect_radius_factor = if show_bisect {
        let input = prompt("Radius factor (default 2.0): ");
        input.parse::<f64>().unwrap_or(2.0)
    } else {
        2.0
    };

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
                            },
                            point2: PointDefinition {
                                kml: kml_file.clone(),
                                name: pair[1].clone(),
                                color: None,
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

    let input_data = InputData { choices };

    // Write debug YAML
    let debug_yml = "rer-debug-input.yml";
    let yml_file = File::create(debug_yml).expect("Failed to create debug YAML");
    serde_yaml::to_writer(yml_file, &input_data).expect("Failed to write debug YAML");
    println!("Debug YAML written to {}", debug_yml);

    // Generate KML directly using processing module
    let output_kml = "rer-out.kml";
    let mut kml_cache: HashMap<String, Kml> = HashMap::new();
    let output_elements = process_choices(&input_data.choices, &mut kml_cache);

    let output_kml_doc = Kml::KmlDocument(KmlDocument {
        elements: vec![Kml::Document {
            attrs: HashMap::new(),
            elements: output_elements,
        }],
        ..Default::default()
    });

    let file = File::create(output_kml).expect("Failed to create output KML file");
    let mut writer = KmlWriter::from_writer(BufWriter::new(file));
    writer.write(&output_kml_doc).expect("Failed to write KML");
    println!("Written to {}", output_kml);
}
