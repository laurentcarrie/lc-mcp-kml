use lc_kml_utils::model::*;

#[test]
fn test_input_data() {
    let input = InputData {
        choices: vec![EChoice::ConcentricCircles(ConcentricCircles {
            center: PointDefinition {
                kml: "blah.kml".to_string(),
                name: "maison".to_string(),
            },
            name: "test".to_string(),
            v_radius: vec![500.0, 1000.0],
        })],
    };
    assert_eq!(input.choices.len(), 1);
}
