#[test]
fn billboard_shader_is_valid_wgsl() {
    let module =
        naga::front::wgsl::parse_str(belfast_experiment_forest_walk::scene::SHADER).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap();
}
