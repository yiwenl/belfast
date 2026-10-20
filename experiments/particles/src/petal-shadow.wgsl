@vertex fn vs_shadow_instances(@location(0) position: vec3<f32>, @builtin(instance_index) index: u32) -> @builtin(position) vec4<f32> {
    return shadow_light.view_projection * vec4(petal_world_position(position, petals[index]), 1.);
}
