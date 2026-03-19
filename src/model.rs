use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct PointDefinition {
    #[serde(default)]
    pub kml: String,
    #[serde(default)]
    pub name: String,
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lat: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lng: Option<f64>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ConcentricCircles {
    pub center: PointDefinition,
    pub name: String,
    pub v_radius: Vec<f64>,
    #[serde(default)]
    pub circle_on_top: bool,
    pub colors: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct UnionCircles {
    pub name: String,
    pub centers: Vec<PointDefinition>,
    pub radius: f64,
    #[serde(default)]
    pub circle_on_top: bool,
    pub color: Option<String>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Segments {
    pub name: String,
    pub kml: String,
    pub neighbours: Vec<[String; 2]>,
    pub color: Option<String>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct TriangleBisect {
    pub point1: PointDefinition,
    pub point2: PointDefinition,
    pub radius_factor: f64,
}

fn default_alpha() -> f64 { 1.0 }

#[derive(Deserialize, Serialize, Clone)]
pub struct RawKml {
    pub path: String,
    pub color: Option<String>,
    #[serde(default = "default_alpha", deserialize_with = "deserialize_alpha")]
    pub alpha: f64,
}

fn deserialize_alpha<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
    let v: Option<f64> = Option::deserialize(deserializer)?;
    Ok(v.unwrap_or(1.0))
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Route {
    pub name: String,
    pub from: PointDefinition,
    pub to: PointDefinition,
    pub color: Option<String>,
    #[serde(default = "default_route_mode")]
    pub mode: String,
}

fn default_route_mode() -> String { "foot".to_string() }

#[derive(Deserialize, Serialize, Clone)]
pub struct BulkRawKml {
    pub prefix: String,
    pub color: Option<String>,
    #[serde(default = "default_alpha", deserialize_with = "deserialize_alpha")]
    pub alpha: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter_commune: Option<String>,
}

#[derive(Deserialize, Serialize, Clone)]
pub enum EChoice {
    ConcentricCircles(ConcentricCircles),
    Point(PointDefinition),
    Folder(Folder),
    UnionCircles(UnionCircles),
    Segments(Segments),
    TriangleBisect(TriangleBisect),
    RawKml(RawKml),
    BulkRawKml(BulkRawKml),
    Route(Route),
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Folder {
    pub name: String,
    pub choices: Vec<EChoice>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct InputData {
    pub choices: Vec<EChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Deserialize)]
pub struct RerLine {
    pub kml: String,
    pub neighbours: Vec<[String; 2]>,
}
