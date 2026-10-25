use belfast_experiment_forest_walk::{camera_path::CameraTrack, point_cloud::PointCloud};

#[test]
#[ignore = "requires local assets; run prepare:assets first"]
fn supplied_forest_cloud_and_reliable_recorded_poses_load() {
    let public = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("public");
    let points = PointCloud::parse(&std::fs::read(public.join("forest.ply")).unwrap()).unwrap();
    let track =
        CameraTrack::from_json(&std::fs::read_to_string(public.join("cameras.json")).unwrap())
            .unwrap();
    assert_eq!(points.points.len(), 2_011_687);
    assert_eq!(track.len(), 151);
    let first = track.sample(0.);
    assert!((first.position.x - 1.6304138).abs() < 1e-5);
    assert!((first.position.z + 4.651715).abs() < 1e-5);
    let forward = first.rotation * -glam::Vec3::Z;
    assert!((forward - glam::Vec3::new(-0.28410575, 0.01868649, 0.95861083)).length() < 1e-5);
}
