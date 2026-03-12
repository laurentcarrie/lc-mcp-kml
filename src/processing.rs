use std::collections::HashMap;
use std::path::Path;

use ::kml::types::{
    AltitudeMode, Coord, Folder as KmlFolder, LineStyle, LinearRing, Placemark, PolyStyle,
    Polygon, Style,
};
use ::kml::{Kml, KmlReader};
use geo::BooleanOps;

use crate::model::EChoice;

pub fn circle_coords(center: &Coord, radius_m: f64, num_points: usize, on_top: bool) -> Vec<Coord> {
    let earth_radius = 6_371_000.0;
    let lat = center.y.to_radians();
    let lon = center.x.to_radians();

    let mut coords = Vec::with_capacity(num_points + 1);
    for i in 0..=num_points {
        let bearing = 2.0 * std::f64::consts::PI * (i as f64) / (num_points as f64);
        let d = radius_m / earth_radius;

        let lat2 = (lat.sin() * d.cos() + lat.cos() * d.sin() * bearing.cos()).asin();
        let lon2 =
            lon + (bearing.sin() * d.sin() * lat.cos()).atan2(d.cos() - lat.sin() * lat2.sin());

        coords.push(Coord {
            x: lon2.to_degrees(),
            y: lat2.to_degrees(),
            z: Some(if on_top { 50.0 } else { 0.0 }),
        });
    }
    coords
}

pub fn circle_geo_polygon(center: &Coord, radius_m: f64, num_points: usize) -> geo::Polygon<f64> {
    let earth_radius = 6_371_000.0;
    let lat = center.y.to_radians();
    let lon = center.x.to_radians();

    let coords: Vec<geo::Coord<f64>> = (0..=num_points)
        .map(|i| {
            let bearing = 2.0 * std::f64::consts::PI * (i % num_points) as f64 / num_points as f64;
            let d = radius_m / earth_radius;
            let lat2 = (lat.sin() * d.cos() + lat.cos() * d.sin() * bearing.cos()).asin();
            let lon2 = lon
                + (bearing.sin() * d.sin() * lat.cos()).atan2(d.cos() - lat.sin() * lat2.sin());
            geo::Coord {
                x: lon2.to_degrees(),
                y: lat2.to_degrees(),
            }
        })
        .collect();
    geo::Polygon::new(geo::LineString(coords), vec![])
}

pub fn multi_polygon_to_kml(
    mp: &geo::MultiPolygon<f64>,
    on_top: bool,
) -> Vec<::kml::types::Geometry> {
    let z = Some(if on_top { 50.0 } else { 0.0 });
    let altitude_mode = if on_top {
        AltitudeMode::RelativeToGround
    } else {
        AltitudeMode::ClampToGround
    };

    mp.0.iter()
        .map(|poly| {
            let outer_coords: Vec<Coord> = poly
                .exterior()
                .0
                .iter()
                .map(|c| Coord {
                    x: c.x,
                    y: c.y,
                    z,
                })
                .collect();
            let inners: Vec<LinearRing> = poly
                .interiors()
                .iter()
                .map(|ring| {
                    let coords: Vec<Coord> = ring
                        .0
                        .iter()
                        .map(|c| Coord {
                            x: c.x,
                            y: c.y,
                            z,
                        })
                        .collect();
                    LinearRing {
                        coords,
                        altitude_mode,
                        ..Default::default()
                    }
                })
                .collect();
            ::kml::types::Geometry::Polygon(Polygon {
                outer: LinearRing {
                    coords: outer_coords,
                    altitude_mode,
                    ..Default::default()
                },
                inner: inners,
                altitude_mode,
                ..Default::default()
            })
        })
        .collect()
}

pub fn find_placemark_point(kml: &Kml, name: &str) -> Option<Coord> {
    match kml {
        Kml::KmlDocument(doc) => doc.elements.iter().find_map(|e| find_placemark_point(e, name)),
        Kml::Document { elements, .. } => {
            elements.iter().find_map(|e| find_placemark_point(e, name))
        }
        Kml::Folder(folder) => {
            folder.elements.iter().find_map(|e| find_placemark_point(e, name))
        }
        Kml::Placemark(p) => {
            if p.name.as_deref() == Some(name) {
                if let Some(::kml::types::Geometry::Point(pt)) = &p.geometry {
                    return Some(pt.coord.clone());
                }
            }
            None
        }
        _ => None,
    }
}

pub fn process_choices(choices: &[EChoice], kml_cache: &mut HashMap<String, Kml>) -> Vec<Kml> {
    let mut elements: Vec<Kml> = Vec::new();

    for choice in choices {
        match choice {
            EChoice::ConcentricCircles(cc) => {
                let kml = kml_cache.entry(cc.center.kml.clone()).or_insert_with(|| {
                    let kml_path = Path::new(&cc.center.kml);
                    KmlReader::from_path(kml_path)
                        .expect(&format!("Failed to open {}", cc.center.kml))
                        .read()
                        .expect("Failed to parse KML")
                });

                let center = find_placemark_point(kml, &cc.center.name)
                    .expect(&format!("Placemark '{}' not found", cc.center.name));

                if let Some(ref colors) = cc.colors {
                    assert_eq!(
                        colors.len(),
                        cc.v_radius.len(),
                        "colors and v_radius must have the same length for '{}'",
                        cc.name
                    );
                }

                for (i, radius) in cc.v_radius.iter().enumerate() {
                    let style_id = format!("circle_style_{}_{}", cc.name, i);
                    let color = cc
                        .colors
                        .as_ref()
                        .map(|c| c[i].clone())
                        .unwrap_or_else(|| "ff0000ff".to_string());

                    let fill_color = format!("1a{}", &color[2..]);
                    elements.push(Kml::Style(Style {
                        id: Some(style_id.clone()),
                        line: Some(LineStyle {
                            color,
                            width: 2.0,
                            ..Default::default()
                        }),
                        poly: Some(PolyStyle {
                            color: fill_color,
                            fill: true,
                            outline: true,
                            ..Default::default()
                        }),
                        ..Default::default()
                    }));

                    let coords = circle_coords(&center, *radius, 72, cc.circle_on_top);
                    let altitude_mode = if cc.circle_on_top {
                        AltitudeMode::RelativeToGround
                    } else {
                        AltitudeMode::ClampToGround
                    };
                    elements.push(Kml::Placemark(Placemark {
                        name: Some(format!("{} - {}m", cc.name, radius)),
                        style_url: Some(format!("#{}", style_id)),
                        geometry: Some(::kml::types::Geometry::Polygon(Polygon {
                            outer: LinearRing {
                                coords,
                                altitude_mode,
                                ..Default::default()
                            },
                            altitude_mode,
                            ..Default::default()
                        })),
                        ..Default::default()
                    }));
                }
            }
            EChoice::Point(pd) => {
                let kml = kml_cache.entry(pd.kml.clone()).or_insert_with(|| {
                    let kml_path = Path::new(&pd.kml);
                    KmlReader::from_path(kml_path)
                        .expect(&format!("Failed to open {}", pd.kml))
                        .read()
                        .expect("Failed to parse KML")
                });

                let coord = find_placemark_point(kml, &pd.name)
                    .expect(&format!("Placemark '{}' not found", pd.name));

                let style_url = pd.color.as_ref().map(|color| {
                    let style_id = format!("point_style_{}", color);
                    elements.push(Kml::Style(Style {
                        id: Some(style_id.clone()),
                        icon: Some(::kml::types::IconStyle {
                            color: color.clone(),
                            icon: ::kml::types::Icon {
                                href: "http://maps.google.com/mapfiles/kml/shapes/rail.png"
                                    .to_string(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                        ..Default::default()
                    }));
                    format!("#{}", style_id)
                });

                elements.push(Kml::Placemark(Placemark {
                    name: Some(pd.name.clone()),
                    style_url,
                    geometry: Some(::kml::types::Geometry::Point(::kml::types::Point {
                        coord,
                        ..Default::default()
                    })),
                    ..Default::default()
                }));
            }
            EChoice::UnionCircles(uc) => {
                let mut union_result: Option<geo::MultiPolygon<f64>> = None;

                for pd in &uc.centers {
                    let kml = kml_cache.entry(pd.kml.clone()).or_insert_with(|| {
                        let kml_path = Path::new(&pd.kml);
                        KmlReader::from_path(kml_path)
                            .expect(&format!("Failed to open {}", pd.kml))
                            .read()
                            .expect("Failed to parse KML")
                    });
                    let center = find_placemark_point(kml, &pd.name)
                        .expect(&format!("Placemark '{}' not found", pd.name));
                    let circle = circle_geo_polygon(&center, uc.radius, 72);

                    union_result = Some(match union_result {
                        None => geo::MultiPolygon(vec![circle]),
                        Some(acc) => acc.union(&circle),
                    });
                }

                if let Some(mp) = union_result {
                    let color = uc
                        .color
                        .clone()
                        .unwrap_or_else(|| "ff0000ff".to_string());
                    let style_id = format!("union_style_{}", uc.name);

                    let fill_color = format!("1a{}", &color[2..]);
                    elements.push(Kml::Style(Style {
                        id: Some(style_id.clone()),
                        line: Some(LineStyle {
                            color,
                            width: 2.0,
                            ..Default::default()
                        }),
                        poly: Some(PolyStyle {
                            color: fill_color,
                            fill: true,
                            outline: true,
                            ..Default::default()
                        }),
                        ..Default::default()
                    }));

                    let geometries = multi_polygon_to_kml(&mp, uc.circle_on_top);
                    for (i, geom) in geometries.into_iter().enumerate() {
                        let name = if i == 0 {
                            format!("{} - {}m union", uc.name, uc.radius)
                        } else {
                            format!("{} - {}m union ({})", uc.name, uc.radius, i + 1)
                        };
                        elements.push(Kml::Placemark(Placemark {
                            name: Some(name),
                            style_url: Some(format!("#{}", style_id)),
                            geometry: Some(geom),
                            ..Default::default()
                        }));
                    }
                }
            }
            EChoice::Segments(seg) => {
                let kml = kml_cache.entry(seg.kml.clone()).or_insert_with(|| {
                    let kml_path = Path::new(&seg.kml);
                    KmlReader::from_path(kml_path)
                        .expect(&format!("Failed to open {}", seg.kml))
                        .read()
                        .expect("Failed to parse KML")
                });

                let color = seg
                    .color
                    .clone()
                    .unwrap_or_else(|| "ff0000ff".to_string());
                let style_id = format!("segments_style_{}", seg.name);

                elements.push(Kml::Style(Style {
                    id: Some(style_id.clone()),
                    line: Some(LineStyle {
                        color,
                        width: 3.0,
                        ..Default::default()
                    }),
                    ..Default::default()
                }));

                for pair in &seg.neighbours {
                    let coord_a = find_placemark_point(kml, &pair[0])
                        .expect(&format!("Placemark '{}' not found", pair[0]));
                    let coord_b = find_placemark_point(kml, &pair[1])
                        .expect(&format!("Placemark '{}' not found", pair[1]));

                    elements.push(Kml::Placemark(Placemark {
                        name: Some(format!("{} → {}", pair[0], pair[1])),
                        style_url: Some(format!("#{}", style_id)),
                        geometry: Some(::kml::types::Geometry::LineString(
                            ::kml::types::LineString {
                                coords: vec![coord_a, coord_b],
                                ..Default::default()
                            },
                        )),
                        ..Default::default()
                    }));
                }
            }
            EChoice::TriangleBisect(tb) => {
                let kml1 = kml_cache
                    .entry(tb.point1.kml.clone())
                    .or_insert_with(|| {
                        KmlReader::from_path(Path::new(&tb.point1.kml))
                            .expect(&format!("Failed to open {}", tb.point1.kml))
                            .read()
                            .expect("Failed to parse KML")
                    });
                let c1 = find_placemark_point(kml1, &tb.point1.name)
                    .expect(&format!("Placemark '{}' not found", tb.point1.name));

                let kml2 = kml_cache
                    .entry(tb.point2.kml.clone())
                    .or_insert_with(|| {
                        KmlReader::from_path(Path::new(&tb.point2.kml))
                            .expect(&format!("Failed to open {}", tb.point2.kml))
                            .read()
                            .expect("Failed to parse KML")
                    });
                let c2 = find_placemark_point(kml2, &tb.point2.name)
                    .expect(&format!("Placemark '{}' not found", tb.point2.name));

                let earth_radius = 6_371_000.0;
                let lat1 = c1.y.to_radians();
                let lat2 = c2.y.to_radians();
                let dlat = (c2.y - c1.y).to_radians();
                let dlon = (c2.x - c1.x).to_radians();
                let a = (dlat / 2.0).sin().powi(2)
                    + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
                let d = 2.0 * earth_radius * a.sqrt().asin();

                let mid_lat = (c1.y + c2.y) / 2.0;
                let cos_mid = mid_lat.to_radians().cos();

                let dx = (c2.x - c1.x).to_radians() * earth_radius * cos_mid;
                let dy = (c2.y - c1.y).to_radians() * earth_radius;
                let d_local = (dx * dx + dy * dy).sqrt();

                let r = tb.radius_factor * d;
                let a_coeff = d_local / 2.0;
                let h = (r * r - a_coeff * a_coeff).sqrt();

                let mx = dx / 2.0;
                let my = dy / 2.0;
                let px = -dy / d_local;
                let py = dx / d_local;

                let ax = mx + h * px;
                let ay = my + h * py;
                let bx = mx - h * px;
                let by = my - h * py;

                let a_lon = c1.x + (ax / (earth_radius * cos_mid)).to_degrees();
                let a_lat = c1.y + (ay / earth_radius).to_degrees();
                let b_lon = c1.x + (bx / (earth_radius * cos_mid)).to_degrees();
                let b_lat = c1.y + (by / earth_radius).to_degrees();

                let style_id = format!(
                    "bisect_style_{}_{}",
                    tb.point1.name, tb.point2.name
                );
                elements.push(Kml::Style(Style {
                    id: Some(style_id.clone()),
                    line: Some(LineStyle {
                        color: "ff000000".to_string(),
                        width: 2.0,
                        ..Default::default()
                    }),
                    ..Default::default()
                }));

                // Simulate dashed line by splitting into segments and drawing every other one
                let num_dashes = 20;
                for i in 0..num_dashes {
                    if i % 2 != 0 {
                        continue; // skip gaps
                    }
                    let t0 = i as f64 / num_dashes as f64;
                    let t1 = (i + 1) as f64 / num_dashes as f64;
                    let lon0 = a_lon + (b_lon - a_lon) * t0;
                    let lat0 = a_lat + (b_lat - a_lat) * t0;
                    let lon1 = a_lon + (b_lon - a_lon) * t1;
                    let lat1 = a_lat + (b_lat - a_lat) * t1;

                    elements.push(Kml::Placemark(Placemark {
                        name: if i == 0 {
                            Some(format!(
                                "Bisect {} - {}",
                                tb.point1.name, tb.point2.name
                            ))
                        } else {
                            None
                        },
                        style_url: Some(format!("#{}", style_id)),
                        geometry: Some(::kml::types::Geometry::LineString(
                            ::kml::types::LineString {
                                coords: vec![
                                    Coord {
                                        x: lon0,
                                        y: lat0,
                                        z: Some(0.0),
                                    },
                                    Coord {
                                        x: lon1,
                                        y: lat1,
                                        z: Some(0.0),
                                    },
                                ],
                                ..Default::default()
                            },
                        )),
                        ..Default::default()
                    }));
                }
            }
            EChoice::Folder(folder) => {
                let folder_elements = process_choices(&folder.choices, kml_cache);
                elements.push(Kml::Folder(KmlFolder {
                    name: Some(folder.name.clone()),
                    elements: folder_elements,
                    ..Default::default()
                }));
            }
            EChoice::RawKml(raw) => {
                let kml = KmlReader::from_path(Path::new(&raw.path))
                    .expect(&format!("Failed to open {}", raw.path))
                    .read()
                    .expect(&format!("Failed to parse {}", raw.path));
                fn extract_elements(kml: Kml) -> Vec<Kml> {
                    match kml {
                        Kml::KmlDocument(doc) => {
                            doc.elements.into_iter().flat_map(extract_elements).collect()
                        }
                        Kml::Document { elements, .. } => elements
                            .into_iter()
                            .filter(|e| !matches!(e, Kml::Element(el) if el.name == "name"))
                            .collect(),
                        other => vec![other],
                    }
                }
                if let Some(ref color) = raw.color {
                    // Override styles with the given color
                    let style_id = format!(
                        "rawkml_color_{}",
                        raw.path.replace(['/', '.', '-'], "_")
                    );
                    let alpha_byte = (raw.alpha.clamp(0.0, 1.0) * 255.0) as u8;
                    let fill_color = format!("{:02x}{}", alpha_byte, &color[2..]);
                    elements.push(Kml::Style(Style {
                        id: Some(style_id.clone()),
                        line: Some(LineStyle {
                            color: color.clone(),
                            width: 2.0,
                            ..Default::default()
                        }),
                        poly: Some(PolyStyle {
                            color: fill_color,
                            fill: true,
                            outline: true,
                            ..Default::default()
                        }),
                        ..Default::default()
                    }));
                    let style_url = format!("#{}", style_id);
                    for el in extract_elements(kml) {
                        match el {
                            Kml::Placemark(mut p) => {
                                p.style_url = Some(style_url.clone());
                                elements.push(Kml::Placemark(p));
                            }
                            other => elements.push(other),
                        }
                    }
                } else {
                    elements.extend(extract_elements(kml));
                }
            }
        }
    }

    elements
}
