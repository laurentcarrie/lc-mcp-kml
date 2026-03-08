use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::BufWriter;

use ::kml::types::KmlDocument;
use ::kml::{Kml, KmlWriter};
use lc_kml_utils::model::InputData;
use lc_kml_utils::processing::process_choices;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <input.yml> <output.kml>", args[0]);
        std::process::exit(1);
    }
    let input_yml = &args[1];
    let output_kml = &args[2];

    let yml_file = File::open(input_yml).expect("Failed to open input YAML file");
    let input_data: InputData = serde_yaml::from_reader(yml_file).expect("Failed to parse YAML");

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
