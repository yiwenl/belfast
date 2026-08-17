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
fn primary_drag_rotates_and_clamps_pitch() {
    let mut control = OrbitalControl::new(OrbitalControlOptions {
        damping: 0.0,
        ..Default::default()
    })
    .unwrap();
    let mut camera = camera();
    control.pointer_down([0.0, 0.0], OrbitalPointerButton::Primary, false);
    control.pointer_move([100.0, 100_000.0], [800.0, 600.0]);
    control.update(1.0 / 60.0, &mut camera);

    assert!(control.eye()[0] > 0.0);
    assert!(control.eye()[1] > 0.99 * control.radius());
    assert!(control.eye()[1] < control.radius());
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
fn scroll_clamps_radius_and_ignores_non_finite_input() {
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
    control.pointer_move([f32::INFINITY, 0.0], [800.0, 600.0]);
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
