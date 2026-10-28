// Vortex segment velocity adapted from dli/vortexspheres (MIT, David Li, 2015).
// See licenses/vortexspheres-LICENSE.txt for the original license.
struct FlowParams {
    shadow_view_projection: mat4x4<f32>,
    origin_radius: vec4<f32>,
    step: vec4<f32>, // dt, time, petal upward acceleration, vortex strength
    filament: vec4<f32>, // core radius, upward drift speed, XZ drift speed, reserved
    counts: vec4<u32>, // petals, rings, vertices per ring, reserved
};
@group(0) @binding(0) var<uniform> flow: FlowParams;
@group(0) @binding(1) var<storage, read> filaments_in: array<vec4<f32>>;

const FILAMENT_LIFETIME: f32 = 6.0;
const CIRCULATION: f32 = 0.003;

fn hash_u32(value: u32) -> u32 {
    var h = value;
    h = (h ^ (h >> 16u)) * 0x7feb352du;
    h = (h ^ (h >> 15u)) * 0x846ca68bu;
    return h ^ (h >> 16u);
}

fn random01(seed: u32) -> f32 {
    return f32(hash_u32(seed) >> 8u) / 16777216.0;
}

fn limit_velocity(v: vec3<f32>, maximum: f32) -> vec3<f32> {
    return v * min(1.0, maximum / max(length(v), 0.000001));
}

fn segment_velocity(start: vec3<f32>, end: vec3<f32>, position: vec3<f32>) -> vec3<f32> {
    let g0 = start - position;
    let g1 = end - position;
    let d = dot(g0, g1);
    let r0 = dot(g0, g0);
    let r1 = dot(g1, g1);
    let a2 = flow.filament.x * flow.filament.x;
    let cross_g = cross(g1, g0);
    let numerator = (d - r0) * inverseSqrt(a2 + r0) + (d - r1) * inverseSqrt(a2 + r1);
    let segment = g1 - g0;
    let denominator = max(a2 * dot(segment, segment) + dot(cross_g, cross_g), 0.00000001);
    return cross_g * (-numerator / denominator);
}

fn vortex_velocity(position: vec3<f32>) -> vec3<f32> {
    var velocity = vec3<f32>(0.0);
    for (var ring = 0u; ring < flow.counts.y; ring += 1u) {
        let base = ring * flow.counts.z;
        let age = filaments_in[base].w;
        let weight = smoothstep(0.0, 0.3, age) * (1.0 - smoothstep(5.0, FILAMENT_LIFETIME, age));
        for (var vertex = 0u; vertex < flow.counts.z; vertex += 1u) {
            let start = filaments_in[base + vertex].xyz;
            let end = filaments_in[base + (vertex + 1u) % flow.counts.z].xyz;
            velocity += segment_velocity(start, end, position) * weight;
        }
    }
    return limit_velocity(velocity * CIRCULATION * flow.step.w, 0.6);
}
