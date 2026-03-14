use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default)]
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

#[derive(Deserialize, Serialize)]
pub struct ConcentricCircles {
    pub center: PointDefinition,
    pub name: String,
    pub v_radius: Vec<f64>,
    #[serde(default)]
    pub circle_on_top: bool,
    pub colors: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize)]
pub struct UnionCircles {
    pub name: String,
    pub centers: Vec<PointDefinition>,
    pub radius: f64,
    #[serde(default)]
    pub circle_on_top: bool,
    pub color: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct Segments {
    pub name: String,
    pub kml: String,
    pub neighbours: Vec<[String; 2]>,
    pub color: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct TriangleBisect {
    pub point1: PointDefinition,
    pub point2: PointDefinition,
    pub radius_factor: f64,
}

#[derive(Deserialize, Serialize)]
pub struct RawKml {
    pub path: String,
    pub color: Option<String>,
    #[serde(default)]
    pub alpha: f64,
}

#[derive(Deserialize, Serialize)]
pub struct Route {
    pub name: String,
    pub from: PointDefinition,
    pub to: PointDefinition,
    pub color: Option<String>,
    #[serde(default = "default_route_mode")]
    pub mode: String,
}

fn default_route_mode() -> String { "foot".to_string() }

#[derive(Deserialize, Serialize)]
pub enum EChoice {
    ConcentricCircles(ConcentricCircles),
    Point(PointDefinition),
    Folder(Folder),
    UnionCircles(UnionCircles),
    Segments(Segments),
    TriangleBisect(TriangleBisect),
    RawKml(RawKml),
    Route(Route),
}

#[derive(Deserialize, Serialize)]
pub struct Folder {
    pub name: String,
    pub choices: Vec<EChoice>,
}

#[derive(Deserialize, Serialize)]
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
