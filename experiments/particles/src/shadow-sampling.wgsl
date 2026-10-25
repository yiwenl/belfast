struct ShadowUniforms {
    view_projection: mat4x4<f32>,
    params: vec4<f32>, // strength, map size, depth bias, normal bias
}
@group(2) @binding(0) var<uniform> shadow: ShadowUniforms;
@group(2) @binding(1) var shadow_map: texture_depth_2d;
@group(2) @binding(2) var shadow_sampler: sampler_comparison;

fn point_visibility(position: vec3<f32>, normal: vec3<f32>, light_direction: vec3<f32>) -> f32 {
    if shadow.params.x <= 0. { return 1.; }
    let facing_normal = select(-normal, normal, dot(normal, light_direction) >= 0.);
    let coord = shadow.view_projection * vec4(position + facing_normal * shadow.params.w, 1.);
    if coord.w <= 0. || coord.z < 0. || coord.z > coord.w { return 1.; }
    let visibility = sampleShadowPcf3x3(shadow_map, shadow_sampler, coord, shadow.params.y, shadow.params.z);
    return mix(1. - shadow.params.x, 1., visibility);
}
