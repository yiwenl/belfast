use belfast_experiment_courtyard02::markers::{parse_emitters, Quality};

const EXPORT: &[u8] = include_bytes!("../public/flower-emitters.json");

#[test]
fn live_export_keeps_all_candidates_and_applies_blender_to_scene_transform() {
    let file = parse_emitters(EXPORT).unwrap();
    assert_eq!(file.emitters.len(), 312);
    assert_eq!(
        file.emitters
            .iter()
            .filter(|e| e.quality == Quality::Likely)
            .count(),
        215
    );
    let json: serde_json::Value = serde_json::from_slice(EXPORT).unwrap();
    let offset = json["blender_to_scene"]["offset"].as_array().unwrap();
    for (emitter, record) in file
        .emitters
        .iter()
        .zip(json["emitters"].as_array().unwrap())
    {
        let b = record["blender_position"].as_array().unwrap();
        let expected = [
            b[0].as_f64().unwrap(),
            b[2].as_f64().unwrap(),
            -b[1].as_f64().unwrap(),
        ];
        for axis in 0..3 {
            assert!(
                (emitter.position[axis] as f64 - expected[axis] - offset[axis].as_f64().unwrap())
                    .abs()
                    < 1e-6
            );
        }
    }
}

#[test]
fn rejects_wrong_coordinates_counts_duplicates_and_invalid_geometry() {
    let original: serde_json::Value = serde_json::from_slice(EXPORT).unwrap();
    for (key, value) in [
        ("schema_version", serde_json::json!(2)),
        ("coordinate_system", serde_json::json!("blender-world-z-up")),
        ("count", serde_json::json!(311)),
        ("marker_radius", serde_json::json!(0)),
        ("emitters", serde_json::json!([])),
    ] {
        let mut bad = original.clone();
        bad[key] = value;
        assert!(parse_emitters(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
    for (key, value) in [
        ("id", original["emitters"][1]["id"].clone()),
        ("position", serde_json::json!([1e100, 0, 0])),
        ("quality", serde_json::json!("unknown")),
    ] {
        let mut bad = original.clone();
        bad["emitters"][0][key] = value;
        assert!(parse_emitters(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
}

#[test]
#[ignore = "requires the local courtyard PLY asset"]
fn calibration_matches_original_ply_coordinates() {
    let json: serde_json::Value = serde_json::from_slice(EXPORT).unwrap();
    let bytes = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/public/courtyard1.ply"
    ))
    .unwrap();
    let header_end = bytes
        .windows(11)
        .position(|b| b == b"end_header\n")
        .unwrap()
        + 11;
    let offset = json["blender_to_scene"]["offset"].as_array().unwrap();
    for sample in json["calibration"]["samples"].as_array().unwrap() {
        let index = sample["ply_index"].as_u64().unwrap() as usize;
        let raw: Vec<f32> = bytes[header_end + index * 56..][..12]
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
            .collect();
        let b = sample["blender_position"].as_array().unwrap();
        let transformed = [
            b[0].as_f64().unwrap(),
            b[2].as_f64().unwrap(),
            -b[1].as_f64().unwrap(),
        ];
        for axis in 0..3 {
            let actual = raw[axis] as f64 * if axis == 0 { 1. } else { -1. };
            assert!((actual - transformed[axis] - offset[axis].as_f64().unwrap()).abs() < 1e-5);
        }
    }
}
