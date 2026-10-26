use belfast_experiment_forest_walk::{
    camera_path::CameraTrack,
    overview::{Bounds, FrustumInstance, OverviewCamera},
};
use glam::{Mat4, Vec3};

#[test]
fn elevated_view_fits_every_corner_at_portrait_and_landscape_aspects() {
    let bounds = Bounds::new(Vec3::new(-1.6, -1.4, -4.8), Vec3::new(1.8, 0.3, 3.4));
    for aspect in [0.25, 9. / 16., 1., 2.5] {
        let camera = OverviewCamera::fit(bounds, aspect);
        let offset = camera.position - camera.target;
        assert!(((-offset.y).atan2(offset.xz().length()).to_degrees() - 45.).abs() < 1e-4);
        for corner in bounds.corners() {
            let clip = camera.view_projection.project_point3(corner);
            assert!(clip.is_finite());
            assert!(
                clip.x.abs() < 0.95 && clip.y.abs() < 0.95,
                "{aspect}: {clip:?}"
            );
            assert!((0. ..1.).contains(&clip.z));
        }
    }
}

#[test]
fn frustum_recovers_camera_fov_orientation_and_position() {
    let json = r#"{"cameras":[{"camera_id":1,"width":400,"height":800,"named_parameters":{"fy":400}}],"frames":[{"camera_id":1,"timestamp_seconds":0,"camera_to_world":[[1,0,0,2],[0,1,0,3],[0,0,1,4],[0,0,0,1]]}]}"#;
    let track = CameraTrack::from_json(json).unwrap();
    let pose = track.sample(0.);
    assert_eq!(pose.aspect, 0.5);
    let instance = FrustumInstance::new(pose, pose.aspect, 0.2, [0., 1., 1., 1.]);
    let inverse = Mat4::from_cols_array(&instance.inverse_view_projection);
    let far_center = inverse.project_point3(Vec3::Z);
    assert!((far_center - Vec3::new(2., 3., 4.2)).length() < 1e-4);
    let corner = inverse.project_point3(Vec3::new(1., 1., 1.));
    assert!((corner - Vec3::new(2.1, 2.8, 4.2)).length() < 1e-4);
    let mut nav = belfast_experiment_forest_walk::camera_path::CameraNavigator::new(track);
    nav.look_around(0.7, 0.3);
    let current = FrustumInstance::new(nav.pose(), 0.5, 0.2, [1., 0.5, 0., 1.]);
    let center = Mat4::from_cols_array(&current.inverse_view_projection).project_point3(Vec3::Z);
    let expected = nav.pose().position + nav.pose().rotation * -Vec3::Z * 0.2;
    assert!((center - expected).length() < 1e-4);
}

#[test]
fn a_single_point_still_has_a_finite_overview() {
    let bounds = Bounds::new(Vec3::splat(2.), Vec3::splat(2.));
    let camera = OverviewCamera::fit(bounds, 1.);
    let clip = camera.view_projection.project_point3(Vec3::splat(2.));
    assert!(clip.is_finite());
    assert!(clip.x.abs() < 1e-5 && clip.y.abs() < 1e-5);
    assert!((0. ..1.).contains(&clip.z));
}
use glam::Vec3Swizzles;

#[test]
fn overview_projection_preserves_equal_pixel_scale_on_both_axes() {
    let bounds = Bounds::new(Vec3::splat(-2.), Vec3::splat(2.));
    for (width, height) in [(640., 720.), (405., 720.), (1200., 500.)] {
        let camera = OverviewCamera::fit(bounds, width / height);
        let basis = OverviewCamera::orientation().inverse();
        let center = camera.view_projection.project_point3(camera.target);
        let right = camera
            .view_projection
            .project_point3(camera.target + basis.transform_vector3(Vec3::X));
        let up = camera
            .view_projection
            .project_point3(camera.target + basis.transform_vector3(Vec3::Y));
        let pixel_x = (right.x - center.x).abs() * width;
        let pixel_y = (up.y - center.y).abs() * height;
        assert!(
            (pixel_x - pixel_y).abs() < 0.001,
            "{width}x{height}: {pixel_x} != {pixel_y}"
        );
    }
}
