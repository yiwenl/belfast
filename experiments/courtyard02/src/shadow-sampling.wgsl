struct ShadowUniforms {
    view_projection: mat4x4<f32>,
    params: vec4<f32>, // strength, map size, depth bias, normal bias
    light_direction: vec4<f32>,
};
@group(1) @binding(0) var<uniform> shadow: ShadowUniforms;
@group(1) @binding(1) var shadow_map: texture_depth_2d;
@group(1) @binding(2) var shadow_sampler: sampler_comparison;

fn shadow_visibility(position: vec3<f32>) -> f32 {
    if (shadow.params.x <= 0.0) { return 1.0; }
    let coord = shadow.view_projection * vec4<f32>(position, 1.0);
    // Most courtyard splats are outside the petal footprint and need no texture lookups.
    if (coord.w <= 0.0 || coord.z < 0.0 || coord.z > coord.w || any(abs(coord.xy) > vec2<f32>(coord.w))) {
        return 1.0;
    }
    let visibility = sampleShadowPcf3x3(shadow_map, shadow_sampler, coord, shadow.params.y, shadow.params.z);
    return mix(1.0 - shadow.params.x, 1.0, visibility);
}
