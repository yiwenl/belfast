#[test]
fn looping_billboard_shader_is_valid() {
    let module =
        naga::front::wgsl::parse_str(belfast_experiment_infinite_walk::scene::SHADER).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap();
}
