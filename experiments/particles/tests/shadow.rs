use belfast_experiment_particles::{
    model_data::ModelData,
    petals::{motion_bounds, parse_emitters},
    scene::{COURTYARD_LIGHT, MAGNOLIA_BASE, MAGNOLIA_HEIGHT},
    shadow::fit_shadow_camera,
};
use glam::Vec3;

#[test]
fn shadow_camera_covers_compact_petal_envelope_below_the_light() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/public/");
    let mut branches =
        ModelData::parse(&std::fs::read(format!("{root}magnolia-branches.glb")).unwrap()).unwrap();
    let transform = branches
        .placement_transform(MAGNOLIA_BASE, MAGNOLIA_HEIGHT)
        .unwrap();
    branches.place(MAGNOLIA_BASE, MAGNOLIA_HEIGHT).unwrap();
    let petals =
        ModelData::parse(&std::fs::read(format!("{root}magnolia-petal.glb")).unwrap()).unwrap();
    let emitters =
        parse_emitters(&std::fs::read(format!("{root}magnolia-emitter-positions.json")).unwrap())
            .unwrap();
    let (min, max) = motion_bounds(&emitters, transform, &petals);
    let (tree_min, tree_max) = branches.bounds();
    let min = min.min(tree_min);
    let max = max.max(tree_max);
    assert!(
        (max - min).max_element() < 9.,
        "Shadow envelope wastes resolution on empty space: {:?}",
        max - min
    );
    let light = Vec3::from(COURTYARD_LIGHT.position);
    assert!(
        light.y > max.y + 5.,
        "Light {light:?} must stay above the shadow envelope {max:?}"
    );
    let vp = fit_shadow_camera(light, min, max).unwrap();
    for x in [min.x, max.x] {
        for y in [min.y, max.y] {
            for z in [min.z, max.z] {
                let clip = vp * Vec3::new(x, y, z).extend(1.);
                assert!(clip.w > 0.);
                let ndc = clip.truncate() / clip.w;
                assert!(
                    ndc.x.abs() < 1. && ndc.y.abs() < 1. && ndc.z > 0. && ndc.z < 1.,
                    "Clipped flight bound {ndc:?}"
                );
            }
        }
    }
    assert!(fit_shadow_camera((min + max) * 0.5, min, max).is_err());
}
