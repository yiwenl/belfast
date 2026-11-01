use belfast::filament::FilamentSettings;

#[test]
fn filament_settings_reject_invalid_geometry_and_motion() {
    let defaults = FilamentSettings::default();
    assert!(defaults.validate().is_ok());
    for invalid in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::MAX] {
        assert!(FilamentSettings {
            ring_radius: invalid,
            ..defaults
        }
        .validate()
        .is_err());
        assert!(FilamentSettings {
            core_radius: invalid,
            ..defaults
        }
        .validate()
        .is_err());
        assert!(FilamentSettings {
            lifetime: invalid,
            ..defaults
        }
        .validate()
        .is_err());
    }
    for invalid in [-1.0, f32::NAN, f32::INFINITY] {
        assert!(FilamentSettings {
            updraft: invalid,
            ..defaults
        }
        .validate()
        .is_err());
        assert!(FilamentSettings {
            xz_drift: invalid,
            ..defaults
        }
        .validate()
        .is_err());
        assert!(FilamentSettings {
            strength: invalid,
            ..defaults
        }
        .validate()
        .is_err());
        assert!(FilamentSettings {
            circulation: invalid,
            ..defaults
        }
        .validate()
        .is_err());
    }
    assert!(FilamentSettings {
        origin: [f32::NAN, 0.0, 0.0],
        ..defaults
    }
    .validate()
    .is_err());
    assert!(FilamentSettings {
        updraft: 0.0,
        xz_drift: 0.0,
        strength: 0.0,
        circulation: 0.0,
        ..defaults
    }
    .validate()
    .is_ok());
}
