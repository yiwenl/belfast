use belfast::{
    BelfastError, OrbitalControl, OrbitalControlOptions, OrbitalPointerButton, PerspectiveCamera,
};

fn assert_approx_eq(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.0001,
        "expected {expected}, got {actual}"
    );
}

fn camera() -> PerspectiveCamera {
    PerspectiveCamera::new(std::f32::consts::FRAC_PI_4, 1.0, 0.1, 100.0)
}

#[test]
fn orbital_control_defaults_to_positive_z() {
    let mut control = OrbitalControl::new(OrbitalControlOptions::default()).unwrap();
    let mut camera = camera();
    control.update(0.0, &mut camera);

    assert_eq!(control.center(), [0.0, 0.0, 0.0]);
    assert_eq!(control.eye(), [0.0, 0.0, 10.0]);
    assert_eq!(camera.look_at_target(), [0.0, 0.0, 0.0]);
}

#[test]
fn orbital_control_rejects_invalid_radius_range() {
    let result = OrbitalControl::new(OrbitalControlOptions {
        min_radius: 2.0,
        max_radius: 1.0,
        ..Default::default()
    });

    assert!(matches!(
        result,
        Err(BelfastError::InvalidOrbitalControlOption("max_radius"))
    ));
}

#[test]
fn orbital_control_rejects_each_invalid_numeric_option() {
    let invalid_options = [
        (
            "center",
            OrbitalControlOptions {
                center: [f32::NAN, 0.0, 0.0],
                ..Default::default()
            },
        ),
        (
            "radius",
            OrbitalControlOptions {
                radius: f32::NAN,
                ..Default::default()
            },
        ),
        (
            "min_radius",
            OrbitalControlOptions {
                min_radius: f32::NEG_INFINITY,
                ..Default::default()
            },
        ),
        (
            "max_radius",
            OrbitalControlOptions {
                max_radius: f32::INFINITY,
                ..Default::default()
            },
        ),
        (
            "rotate_sensitivity",
            OrbitalControlOptions {
                rotate_sensitivity: -1.0,
                ..Default::default()
            },
        ),
        (
            "rotate_sensitivity",
            OrbitalControlOptions {
                rotate_sensitivity: f32::NAN,
                ..Default::default()
            },
        ),
        (
            "rotate_sensitivity",
            OrbitalControlOptions {
                rotate_sensitivity: f32::INFINITY,
                ..Default::default()
            },
        ),
        (
            "rotate_sensitivity",
            OrbitalControlOptions {
                rotate_sensitivity: f32::NEG_INFINITY,
                ..Default::default()
            },
        ),
        (
            "zoom_sensitivity",
            OrbitalControlOptions {
                zoom_sensitivity: -1.0,
                ..Default::default()
            },
        ),
        (
            "zoom_sensitivity",
            OrbitalControlOptions {
                zoom_sensitivity: f32::NAN,
                ..Default::default()
            },
        ),
        (
            "zoom_sensitivity",
            OrbitalControlOptions {
                zoom_sensitivity: f32::INFINITY,
                ..Default::default()
            },
        ),
        (
            "zoom_sensitivity",
            OrbitalControlOptions {
                zoom_sensitivity: f32::NEG_INFINITY,
                ..Default::default()
            },
        ),
        (
            "pan_sensitivity",
            OrbitalControlOptions {
                pan_sensitivity: -1.0,
                ..Default::default()
            },
        ),
        (
            "pan_sensitivity",
            OrbitalControlOptions {
                pan_sensitivity: f32::NAN,
                ..Default::default()
            },
        ),
        (
            "pan_sensitivity",
            OrbitalControlOptions {
                pan_sensitivity: f32::INFINITY,
                ..Default::default()
            },
        ),
        (
            "pan_sensitivity",
            OrbitalControlOptions {
                pan_sensitivity: f32::NEG_INFINITY,
                ..Default::default()
            },
        ),
        (
            "damping",
            OrbitalControlOptions {
                damping: -1.0,
                ..Default::default()
            },
        ),
        (
            "damping",
            OrbitalControlOptions {
                damping: f32::NAN,
                ..Default::default()
            },
        ),
        (
            "damping",
            OrbitalControlOptions {
                damping: f32::INFINITY,
                ..Default::default()
            },
        ),
        (
            "damping",
            OrbitalControlOptions {
                damping: f32::NEG_INFINITY,
                ..Default::default()
            },
        ),
    ];

    for (expected, options) in invalid_options {
        let result = OrbitalControl::new(options);
        assert!(
            matches!(
                result,
                Err(BelfastError::InvalidOrbitalControlOption(actual)) if actual == expected
            ),
            "expected invalid option {expected}, got {result:?}"
        );
    }
}

#[test]
fn primary_horizontal_drag_rotates_toward_negative_x() {
    let mut control = OrbitalControl::new(OrbitalControlOptions {
        damping: 0.0,
        ..Default::default()
    })
    .unwrap();
    let mut camera = camera();
    control.pointer_down([0.0, 0.0], OrbitalPointerButton::Primary, false);
    control.pointer_move([100.0, 0.0], [800.0, 600.0]);
    control.update(1.0 / 60.0, &mut camera);

    assert!(control.eye()[0] < 0.0);
}

#[test]
fn primary_vertical_drag_clamps_pitch() {
    let mut control = OrbitalControl::new(OrbitalControlOptions {
        damping: 0.0,
        ..Default::default()
    })
    .unwrap();
    let mut camera = camera();
    control.pointer_down([0.0, 0.0], OrbitalPointerButton::Primary, false);
    control.pointer_move([0.0, 100_000.0], [800.0, 600.0]);
    control.update(1.0 / 60.0, &mut camera);

    assert!(control.eye()[1] > 0.99 * control.radius());
    assert!(control.eye()[1] < control.radius());
}

#[test]
fn extreme_rotation_keeps_pose_and_camera_finite() {
    let mut control = OrbitalControl::new(OrbitalControlOptions {
        rotate_sensitivity: f32::MAX,
        damping: 0.0,
        ..Default::default()
    })
    .unwrap();
    let mut camera = camera();
    control.pointer_down([0.0, 0.0], OrbitalPointerButton::Primary, false);
    control.pointer_move([2.0, 0.0], [800.0, 600.0]);
    control.update(1.0 / 60.0, &mut camera);

    assert!(control.center().iter().all(|value| value.is_finite()));
    assert!(control.eye().iter().all(|value| value.is_finite()));
    assert!(camera
        .view_projection_matrix()
        .iter()
        .all(|value| value.is_finite()));
}

#[test]
fn shift_primary_drag_pans_the_target() {
    let mut control = OrbitalControl::new(OrbitalControlOptions {
        damping: 0.0,
        ..Default::default()
    })
    .unwrap();
    let mut camera = camera();
    control.pointer_down([0.0, 0.0], OrbitalPointerButton::Primary, true);
    control.pointer_move([60.0, 30.0], [800.0, 600.0]);
    control.update(1.0 / 60.0, &mut camera);

    assert!(control.center()[0] < 0.0);
    assert!(control.center()[1] > 0.0);
    assert_eq!(camera.look_at_target(), control.center());
}

#[test]
fn extreme_pan_keeps_pose_and_camera_finite() {
    let mut control = OrbitalControl::new(OrbitalControlOptions {
        pan_sensitivity: f32::MAX,
        damping: 0.0,
        ..Default::default()
    })
    .unwrap();
    let mut camera = camera();
    control.pointer_down([0.0, 0.0], OrbitalPointerButton::Primary, true);
    control.pointer_move([2.0, 0.0], [800.0, 600.0]);
    control.update(1.0 / 60.0, &mut camera);

    assert!(control.center().iter().all(|value| value.is_finite()));
    assert!(control.eye().iter().all(|value| value.is_finite()));
    assert!(camera
        .view_projection_matrix()
        .iter()
        .all(|value| value.is_finite()));
}

#[test]
fn active_drag_ignores_non_finite_pointer_movement() {
    let mut control = OrbitalControl::new(OrbitalControlOptions {
        damping: 0.0,
        ..Default::default()
    })
    .unwrap();
    let mut camera = camera();
    control.update(0.0, &mut camera);
    let initial_center = control.center();
    let initial_eye = control.eye();
    let initial_view_projection = camera.view_projection_matrix();

    control.pointer_down([0.0, 0.0], OrbitalPointerButton::Primary, false);
    control.pointer_move([f32::INFINITY, 20.0], [800.0, 600.0]);
    control.update(1.0 / 60.0, &mut camera);

    assert_eq!(control.center(), initial_center);
    assert_eq!(control.eye(), initial_eye);
    assert_eq!(camera.view_projection_matrix(), initial_view_projection);
}

#[test]
fn active_drag_ignores_zero_width_viewport() {
    let mut control = OrbitalControl::new(OrbitalControlOptions {
        damping: 0.0,
        ..Default::default()
    })
    .unwrap();
    let mut camera = camera();
    let initial_eye = control.eye();
    control.pointer_down([0.0, 0.0], OrbitalPointerButton::Primary, false);
    control.pointer_move([100.0, 0.0], [0.0, 600.0]);
    control.update(1.0 / 60.0, &mut camera);

    assert_eq!(control.eye(), initial_eye);
}

#[test]
fn active_drag_ignores_zero_height_viewport() {
    let mut control = OrbitalControl::new(OrbitalControlOptions {
        damping: 0.0,
        ..Default::default()
    })
    .unwrap();
    let mut camera = camera();
    let initial_eye = control.eye();
    control.pointer_down([0.0, 0.0], OrbitalPointerButton::Primary, false);
    control.pointer_move([100.0, 0.0], [800.0, 0.0]);
    control.update(1.0 / 60.0, &mut camera);

    assert_eq!(control.eye(), initial_eye);
}

#[test]
fn moderate_scroll_uses_exponential_zoom() {
    let mut control = OrbitalControl::new(OrbitalControlOptions {
        radius: 2.0,
        min_radius: 1.0,
        max_radius: 10.0,
        zoom_sensitivity: 0.25,
        damping: 0.0,
        ..Default::default()
    })
    .unwrap();
    let mut camera = camera();
    control.scroll(2.0);
    control.update(1.0 / 60.0, &mut camera);

    assert_approx_eq(control.radius(), 2.0 * (2.0_f32 * 0.25).exp());
}

#[test]
fn scroll_clamps_radius_and_ignores_non_finite_delta() {
    let mut control = OrbitalControl::new(OrbitalControlOptions {
        radius: 2.0,
        min_radius: 1.0,
        max_radius: 3.0,
        damping: 0.0,
        ..Default::default()
    })
    .unwrap();
    let mut camera = camera();
    control.scroll(-100_000.0);
    control.update(1.0 / 60.0, &mut camera);
    assert_approx_eq(control.radius(), 1.0);

    control.scroll(100_000.0);
    control.scroll(f32::NAN);
    control.update(1.0 / 60.0, &mut camera);
    assert_approx_eq(control.radius(), 3.0);
}

#[test]
fn damping_depends_on_elapsed_time_not_frame_count() {
    fn simulate(step: f32, count: usize) -> [f32; 3] {
        let mut control = OrbitalControl::new(OrbitalControlOptions::default()).unwrap();
        let mut camera = camera();
        control.pointer_down([0.0, 0.0], OrbitalPointerButton::Primary, false);
        control.pointer_move([80.0, 20.0], [800.0, 600.0]);
        for _ in 0..count {
            control.update(step, &mut camera);
        }
        control.eye()
    }

    let sixty_hz = simulate(1.0 / 60.0, 60);
    let thirty_hz = simulate(1.0 / 30.0, 30);
    for index in 0..3 {
        assert_approx_eq(sixty_hz[index], thirty_hz[index]);
    }
}

#[test]
fn damping_uses_shortest_path_across_yaw_wrap() {
    let mut control = OrbitalControl::new(OrbitalControlOptions::default()).unwrap();
    let mut camera = camera();

    control.pointer_down([0.0, 0.0], OrbitalPointerButton::Primary, false);
    control.pointer_move([-313.0, 0.0], [800.0, 600.0]);
    for _ in 0..120 {
        control.update(1.0 / 60.0, &mut camera);
    }
    control.pointer_up(OrbitalPointerButton::Primary);

    control.pointer_down([0.0, 0.0], OrbitalPointerButton::Primary, false);
    control.pointer_move([-2.0, 0.0], [800.0, 600.0]);
    control.update(1.0 / 60.0, &mut camera);

    let eye = control.eye();
    assert!(
        eye[2] < 0.0,
        "camera must remain behind the target: {eye:?}"
    );
    assert!(
        eye[0].abs() < 2.0,
        "small seam-crossing drag must not take the long path: {eye:?}"
    );
}

#[test]
fn set_yaw_animates_toward_target_with_damping() {
    let mut control = OrbitalControl::new(OrbitalControlOptions::default()).unwrap();
    let mut camera = camera();
    control.set_yaw(std::f32::consts::FRAC_PI_2);
    for _ in 0..180 {
        control.update(1.0 / 60.0, &mut camera);
    }

    assert_approx_eq(control.yaw(), std::f32::consts::FRAC_PI_2);
    assert!(control.eye()[0] > 9.0);
    assert_eq!(camera.position(), control.eye());
}

#[test]
fn disabled_control_ignores_pointer_drag() {
    let mut control = OrbitalControl::new(OrbitalControlOptions {
        damping: 0.0,
        ..Default::default()
    })
    .unwrap();
    let mut camera = camera();
    control.update(0.0, &mut camera);
    let initial_eye = control.eye();

    control.set_enabled(false);
    control.pointer_down([0.0, 0.0], OrbitalPointerButton::Primary, false);
    control.pointer_move([100.0, 0.0], [800.0, 600.0]);
    control.scroll(20.0);
    control.update(1.0 / 60.0, &mut camera);

    assert!(!control.enabled());
    assert_eq!(control.eye(), initial_eye);
}

#[test]
fn disabled_control_still_accepts_programmatic_yaw() {
    let mut control = OrbitalControl::new(OrbitalControlOptions::default()).unwrap();
    let mut camera = camera();
    control.set_enabled(false);
    control.set_yaw(std::f32::consts::FRAC_PI_2);
    for _ in 0..180 {
        control.update(1.0 / 60.0, &mut camera);
    }

    assert_approx_eq(control.yaw(), std::f32::consts::FRAC_PI_2);
}

#[test]
fn snap_pitch_applies_immediately_and_clamps() {
    let mut control = OrbitalControl::new(OrbitalControlOptions {
        damping: 0.0,
        ..Default::default()
    })
    .unwrap();
    let mut camera = camera();
    control.snap_pitch(100.0);
    control.update(0.0, &mut camera);

    assert!(control.pitch() > 1.57);
    assert!(control.pitch() < std::f32::consts::FRAC_PI_2);
    assert!(control.eye()[1] > 0.99 * control.radius());
}

#[test]
fn programmatic_yaw_cancels_active_drag() {
    let mut control = OrbitalControl::new(OrbitalControlOptions {
        damping: 0.0,
        ..Default::default()
    })
    .unwrap();
    let mut camera = camera();
    control.pointer_down([0.0, 0.0], OrbitalPointerButton::Primary, false);
    control.set_yaw(std::f32::consts::FRAC_PI_2);
    control.pointer_move([100.0, 0.0], [800.0, 600.0]);
    control.update(1.0 / 60.0, &mut camera);

    assert_approx_eq(control.yaw(), std::f32::consts::FRAC_PI_2);
}
