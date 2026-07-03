use belfast::{OrthographicCamera, PerspectiveCamera};

fn assert_approx_eq(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.0001,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn perspective_camera_updates_view_projection_matrix() {
    let mut camera = PerspectiveCamera::new(std::f32::consts::FRAC_PI_4, 1.0, 0.1, 100.0);
    camera.look_at([0.0, 0.0, 2.0], [0.0, 0.0, 0.0]);

    let view_proj = camera.view_projection_matrix();

    assert_eq!(view_proj.len(), 16);
    assert_approx_eq(view_proj[0], 2.4142134);
    assert!(view_proj[10] < 0.0);
    assert!(view_proj[14] > 0.0);
    assert_eq!(camera.look_at_target(), [0.0, 0.0, 0.0]);
}

#[test]
fn perspective_camera_aspect_can_be_changed() {
    let mut camera = PerspectiveCamera::new(std::f32::consts::FRAC_PI_4, 1.0, 0.1, 100.0);
    let before = camera.projection_matrix()[0];

    camera.set_aspect(2.0).unwrap();

    assert!(camera.projection_matrix()[0] < before);
}

#[test]
fn orthographic_camera_produces_column_major_matrix() {
    let mut camera = OrthographicCamera::new(-2.0, 2.0, -1.0, 1.0, 0.1, 10.0);
    camera.look_at([0.0, 0.0, 2.0], [0.0, 0.0, 0.0]);

    let projection = camera.projection_matrix();
    let view_proj = camera.view_projection_matrix();

    assert_approx_eq(projection[0], 0.5);
    assert_approx_eq(projection[5], 1.0);
    assert_eq!(view_proj.len(), 16);
}
