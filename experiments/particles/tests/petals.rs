use belfast_experiment_particles::{
    model_data::ModelData,
    petals::{initial_particles, parse_emitters, PARTICLE_COUNT},
};
use glam::{Mat4, Vec3};

fn asset(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/public/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

#[test]
fn petal_asset_and_emitters_share_the_branch_placement() {
    let petal = ModelData::parse(&asset("magnolia-petal.glb")).unwrap();
    assert_eq!(petal.primitives.len(), 1);
    assert_eq!(petal.primitives[0].vertices.len(), 45);
    assert_eq!(petal.primitives[0].indices.len(), 192);
    assert!(petal.primitives[0].double_sided);
    let emitters = parse_emitters(&asset("magnolia-emitter-positions.json")).unwrap();
    assert_eq!(emitters.len(), 1435);
    let mut tree = ModelData::parse(&asset("magnolia-branches.glb")).unwrap();
    let transform = tree
        .placement_transform(Vec3::new(0.05, -0.80, -5.98), 2.8)
        .unwrap();
    let position = transform.transform_point3(emitters[0]);
    // Independently calculated from the actual branch bounds and emitter 0.
    assert!((position - Vec3::new(-0.39393, 0.83667, -6.02958)).length() < 0.0001);
    let original = tree.primitives[0].vertices[10].position;
    tree.place(Vec3::new(0.05, -0.80, -5.98), 2.8).unwrap();
    assert!(
        (transform.transform_point3(original.into())
            - Vec3::from(tree.primitives[0].vertices[10].position))
        .length()
            < 1e-5
    );
}

#[test]
fn initializes_ten_thousand_near_emitters_with_independent_lives_and_axes() {
    let emitters = [Vec3::new(1., 2., 3.), Vec3::new(-1., 4., 2.)];
    let transform = Mat4::from_scale_rotation_translation(
        Vec3::splat(0.5),
        glam::Quat::IDENTITY,
        Vec3::new(0., -1., -6.),
    );
    let particles = initial_particles(&emitters, transform, PARTICLE_COUNT).unwrap();
    assert_eq!(particles.len(), 10_000);
    for (i, p) in particles.iter().enumerate() {
        let expected = if i % 2 == 0 {
            Vec3::new(0.5, 0., -4.5)
        } else {
            Vec3::new(-0.5, 1., -5.)
        };
        let origin = Vec3::from_slice(&p.origin_lifetime);
        assert!((origin - expected).length() < 0.04);
        assert_eq!(p.position_life[..3], p.origin_lifetime[..3]);
        assert!(p.position_life[3] > 0. && p.position_life[3] <= p.origin_lifetime[3]);
        let age = 1. - p.position_life[3] / p.origin_lifetime[3];
        assert!((0.0..=1.0).contains(&p.appearance[2]));
        if (0.1..=0.8).contains(&age) {
            assert_eq!(
                p.appearance[2], 1.,
                "Midlife petals must already be visible before the first compute frame"
            );
        }
        assert!((Vec3::from_slice(&p.axis_speed).length() - 1.).abs() < 1e-5);
        assert!(p.axis_speed[3].abs() > 0.);
        assert!(p.appearance[0] > 0. && p.appearance[3] > 0.);
    }
    let held = particles
        .iter()
        .filter(|p| p.position_life[3] / p.origin_lifetime[3] >= 0.8)
        .count();
    assert!(
        (1500..2500).contains(&held),
        "Initial phases should include both held and moving petals: {held}"
    );
    assert!(particles.iter().any(|p| p.appearance[0] < 0.3));
    assert!(particles.iter().any(|p| p.appearance[0] > 0.75));
    assert!(particles.iter().any(|p| p.appearance[3] < 0.7));
    assert!(particles.iter().any(|p| p.appearance[3] > 1.5));
    assert_ne!(particles[0].axis_speed, particles[1].axis_speed);
    assert_ne!(particles[0].position_life[3], particles[1].position_life[3]);
    assert!(initial_particles(&[], Mat4::IDENTITY, 10).is_err());
}

#[test]
fn rejects_misaligned_or_empty_emitter_data() {
    let bytes = asset("magnolia-emitter-positions.json");
    let valid = String::from_utf8(bytes).unwrap();
    assert!(parse_emitters(valid.replace("\"+Y\"", "\"+Z\"").as_bytes()).is_err());
    assert!(parse_emitters(b"{\"emitters\":[]}").is_err());
    assert!(parse_emitters(b"broken JSON").is_err());
}
