use belfast::{BelfastError, OrthographicCamera, PerspectiveCamera};

fn assert_approx_eq(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.0001,
        "expected {expected}, got {actual}"
    );
}

fn multiply_mat4(left: &[f32; 16], right: &[f32; 16]) -> [f32; 16] {
    let mut result = [0.0; 16];
    for column in 0..4 {
        for row in 0..4 {
            result[column * 4 + row] = (0..4)
                .map(|index| left[index * 4 + row] * right[column * 4 + index])
                .sum();
        }
    }
    result
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
fn perspective_camera_exposes_pose_and_projection_parts() {
    let mut camera = PerspectiveCamera::new(std::f32::consts::FRAC_PI_4, 1.0, 0.1, 100.0);
    camera.look_at([0.0, 1.5, 2.0], [0.0, 0.0, 0.0]);

    let view = camera.view_matrix();
    let projection = camera.projection_matrix();
    let expected = multiply_mat4(&projection, &view);

    assert_eq!(camera.position(), [0.0, 1.5, 2.0]);
    assert_eq!(camera.fovy_radians(), std::f32::consts::FRAC_PI_4);
    assert_eq!(camera.aspect(), 1.0);
    assert_eq!(camera.near(), 0.1);
    assert_eq!(camera.far(), 100.0);
    assert_eq!(view.len(), 16);
    assert_eq!(projection.len(), 16);
    for (actual, expected) in camera.view_projection_matrix().iter().zip(expected) {
        assert_approx_eq(*actual, expected);
    }
}

#[test]
fn perspective_camera_aspect_can_be_changed() {
    let mut camera = PerspectiveCamera::new(std::f32::consts::FRAC_PI_4, 1.0, 0.1, 100.0);
    let before = camera.projection_matrix()[0];

    camera.set_aspect(2.0).unwrap();

    assert_eq!(camera.aspect(), 2.0);
    assert!(camera.projection_matrix()[0] < before);
}

#[test]
fn perspective_camera_fov_can_be_changed() {
    let mut camera = PerspectiveCamera::new(std::f32::consts::FRAC_PI_4, 1.0, 0.1, 100.0);
    let before = camera.projection_matrix()[5];

    camera
        .set_fovy_radians(std::f32::consts::FRAC_PI_2)
        .unwrap();

    assert_eq!(camera.fovy_radians(), std::f32::consts::FRAC_PI_2);
    assert!(camera.projection_matrix()[5] < before);
    assert!(matches!(
        camera.set_fovy_radians(0.0),
        Err(BelfastError::InvalidCameraFov)
    ));
}

#[test]
fn orthographic_camera_produces_column_major_matrix() {
    let mut camera = OrthographicCamera::new(-2.0, 2.0, -1.0, 1.0, 0.1, 10.0);
    camera.look_at([0.0, 0.0, 2.0], [0.0, 0.0, 0.0]);

    let projection = camera.projection_matrix();
    let view_proj = camera.view_projection_matrix();

    assert_eq!(camera.position(), [0.0, 0.0, 2.0]);
    assert_approx_eq(projection[0], 0.5);
    assert_approx_eq(projection[5], 1.0);
    assert_eq!(camera.view_matrix().len(), 16);
    assert_eq!(view_proj.len(), 16);
}
