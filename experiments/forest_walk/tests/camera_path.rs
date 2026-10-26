use belfast_experiment_forest_walk::camera_path::{CameraNavigator, CameraTrack};
use glam::Vec3;
use serde_json::json;

fn fixture() -> serde_json::Value {
    json!({
        "undistorted_cameras": [{"camera_id":1,"width":800,"height":800,"named_parameters":{"fx":400.0,"fy":400.0}}],
        "frames": [
            {"camera_id":1,"timestamp_seconds":0.0,"camera_to_world":[[1,0,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,1]]},
            {"camera_id":1,"timestamp_seconds":1.0,"camera_to_world":[[1,0,0,1],[0,1,0,0],[0,0,1,0],[0,0,0,1]]},
            {"camera_id":1,"timestamp_seconds":2.0,"camera_to_world":[[1,0,0,1],[0,1,0,1],[0,0,1,0],[0,0,0,1]]}
        ]
    })
}
fn near(a: Vec3, b: Vec3) {
    assert!((a - b).length() < 1e-5, "{a:?} != {b:?}");
}

#[test]
fn converts_opencv_axes_and_preserves_the_recorded_camera_roll() {
    let track = CameraTrack::from_json(&fixture().to_string()).unwrap();
    let pose = track.sample(0.);
    near(
        pose.view_matrix().transform_point3(Vec3::new(0., 0., 5.)),
        Vec3::new(0., 0., -5.),
    );
    near(pose.rotation * Vec3::Y, -Vec3::Y);
    assert!((pose.fov_y - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
    let mut data = fixture();
    data["frames"][0]["camera_to_world"] =
        json!([[0, -1, 0, 0], [1, 0, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]]);
    let pose = CameraTrack::from_json(&data.to_string())
        .unwrap()
        .sample(0.);
    near(pose.rotation * Vec3::Y, Vec3::X);
}

#[test]
fn interpolation_follows_each_path_segment_instead_of_cutting_corners() {
    let track = CameraTrack::from_json(&fixture().to_string()).unwrap();
    near(track.sample(0.5).position, Vec3::new(0.5, 0., 0.));
    near(track.sample(1.5).position, Vec3::new(1., 0.5, 0.));
    near(track.sample(-1.).position, Vec3::ZERO);
    near(track.sample(99.).position, Vec3::new(1., 1., 0.));
}

#[test]
fn navigation_is_smooth_reversible_and_clamps_at_both_ends() {
    let mut nav = CameraNavigator::new(CameraTrack::from_json(&fixture().to_string()).unwrap());
    nav.step(-1);
    assert_eq!(nav.index(), 0);
    nav.step(1);
    assert_eq!(nav.index(), 1);
    near(nav.pose().position, Vec3::ZERO);
    nav.advance(0.05);
    assert!(nav.pose().position.x > 0. && nav.pose().position.x < 1.);
    nav.step(1);
    nav.step(1);
    assert_eq!(nav.index(), 2);
    for _ in 0..180 {
        nav.advance(1. / 60.);
    }
    near(nav.pose().position, Vec3::new(1., 1., 0.));
    nav.step(-1);
    for _ in 0..180 {
        nav.advance(1. / 60.);
    }
    near(nav.pose().position, Vec3::new(1., 0., 0.));
}

#[test]
fn sorts_by_recorded_time_and_rejects_invalid_transforms() {
    let mut data = fixture();
    data["frames"].as_array_mut().unwrap().reverse();
    let track = CameraTrack::from_json(&data.to_string()).unwrap();
    near(track.sample(0.).position, Vec3::ZERO);
    data["frames"][0]["camera_to_world"][0][0] = json!(2);
    assert!(CameraTrack::from_json(&data.to_string()).is_err());
    data["frames"] = json!([]);
    assert!(CameraTrack::from_json(&data.to_string()).is_err());
}

#[test]
fn stepping_changes_position_without_replaying_recorded_orientation_or_fov() {
    let mut data = fixture();
    data["undistorted_cameras"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "camera_id":2,"width":800,"height":800,"named_parameters":{"fy":800.0}
        }));
    data["frames"][1]["camera_id"] = json!(2);
    data["frames"][1]["camera_to_world"] =
        json!([[0, 0, 1, 1], [0, 1, 0, 0], [-1, 0, 0, 0], [0, 0, 0, 1]]);
    let mut nav = CameraNavigator::new(CameraTrack::from_json(&data.to_string()).unwrap());
    let initial = nav.pose();
    nav.step(1);
    nav.advance(0.05);
    assert!(nav.pose().position.x > 0. && nav.pose().position.x < 1.);
    near(nav.pose().rotation * -Vec3::Z, Vec3::Z);
    for _ in 0..180 {
        nav.advance(1. / 60.);
    }
    near(nav.pose().position, Vec3::X);
    near(nav.pose().rotation * -Vec3::Z, Vec3::Z);
    near(nav.pose().rotation * Vec3::Y, -Vec3::Y);
    assert_eq!(nav.pose().fov_y, initial.fov_y);
}

#[test]
fn looking_rotates_in_place_and_survives_forward_and_backward_steps() {
    let mut nav = CameraNavigator::new(CameraTrack::from_json(&fixture().to_string()).unwrap());
    nav.look_around(std::f32::consts::FRAC_PI_2, 0.);
    near(nav.pose().position, Vec3::ZERO);
    near(nav.pose().rotation * -Vec3::Z, -Vec3::X);
    for direction in [1, -1] {
        nav.step(direction);
        for _ in 0..180 {
            nav.advance(1. / 60.);
            near(nav.pose().rotation * -Vec3::Z, -Vec3::X);
        }
    }
    near(nav.pose().position, Vec3::ZERO);
}

#[test]
fn looking_clamps_pitch_without_flipping_and_ignores_invalid_deltas() {
    let mut nav = CameraNavigator::new(CameraTrack::from_json(&fixture().to_string()).unwrap());
    nav.look_around(0., 100.);
    let up = nav.pose().rotation * Vec3::Y;
    let forward = nav.pose().rotation * -Vec3::Z;
    assert!(up.dot(-Vec3::Y) > 0.);
    assert!(forward.dot(-Vec3::Y) > 0.99);
    let rotation = nav.pose().rotation;
    nav.look_around(f32::NAN, 0.);
    nav.look_around(0., f32::INFINITY);
    assert_eq!(nav.pose().rotation, rotation);
    nav.look_around(0., -200.);
    assert!((nav.pose().rotation * Vec3::Y).dot(-Vec3::Y) > 0.);
    assert!((nav.pose().rotation * -Vec3::Z).dot(Vec3::Y) > 0.99);
    near(nav.pose().position, Vec3::ZERO);
}

#[test]
fn recording_time_follows_the_same_interpolated_position_in_both_directions() {
    let mut data = fixture();
    for (frame, time) in data["frames"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .zip([1.25, 4.75, 9.0])
    {
        frame["timestamp_seconds"] = json!(time);
    }
    data["frames"].as_array_mut().unwrap().reverse();
    let track = CameraTrack::from_json(&data.to_string()).unwrap();
    assert_eq!(track.sample_time(-1.), 1.25);
    assert_eq!(track.sample_time(f32::NAN), 1.25);
    assert_eq!(track.sample_time(0.5), 3.0);
    assert_eq!(track.sample_time(1.5), 6.875);
    assert_eq!(track.sample_time(99.), 9.0);
    let mut nav = CameraNavigator::new(track);
    nav.step(1);
    nav.advance(0.05);
    assert!(
        (nav.timestamp_seconds() - (1.25 + f64::from(nav.pose().position.x) * 3.5)).abs() < 1e-6
    );
    for _ in 0..180 {
        nav.advance(1. / 60.);
    }
    assert_eq!(nav.timestamp_seconds(), 4.75);
    nav.look_around(0.5, 0.2);
    assert_eq!(nav.timestamp_seconds(), 4.75);
    nav.step(-1);
    nav.advance(0.05);
    assert!(nav.timestamp_seconds() > 1.25 && nav.timestamp_seconds() < 4.75);
    for _ in 0..180 {
        nav.advance(1. / 60.);
    }
    assert_eq!(nav.timestamp_seconds(), 1.25);
}
