struct ShadowLight {
    view_projection: mat4x4<f32>,
    params: vec4<f32>,
}
@group(0) @binding(0) var<uniform> shadow_light: ShadowLight;

@vertex fn vs_shadow(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    return shadow_light.view_projection * vec4(position, 1.);
}
