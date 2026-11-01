struct ShadowLight {
    view_projection: mat4x4<f32>,
};
@group(1) @binding(0) var<uniform> shadow_light: ShadowLight;

@vertex fn vs_main(@location(0) position: vec3<f32>, @builtin(instance_index) index: u32) -> @builtin(position) vec4<f32> {
    return shadow_light.view_projection * vec4<f32>(petal_world_position(position, petals[index]), 1.0);
}

@fragment fn fs_main() {}
