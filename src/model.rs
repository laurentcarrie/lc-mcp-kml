use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct PointDefinition {
    pub kml: String,
    pub name: String,
    pub color: Option<String>,
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
pub enum EChoice {
    ConcentricCircles(ConcentricCircles),
    Point(PointDefinition),
    Folder(Folder),
    UnionCircles(UnionCircles),
    Segments(Segments),
    TriangleBisect(TriangleBisect),
    RawKml(RawKml),
}

#[derive(Deserialize, Serialize)]
pub struct Folder {
    pub name: String,
    pub choices: Vec<EChoice>,
}

#[derive(Deserialize, Serialize)]
pub struct InputData {
    pub choices: Vec<EChoice>,
}

#[derive(Deserialize)]
pub struct RerLine {
    pub kml: String,
    pub neighbours: Vec<[String; 2]>,
}
