// Vortex segment velocity adapted from dli/vortexspheres (MIT, David Li, 2015).
/*
The MIT License (MIT)

Copyright (c) 2015 David Li (http://david.li)

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/
struct FilamentParams {
    origin_radius: vec4<f32>,
    step: vec4<f32>, // dt, time, strength, lifetime
    motion: vec4<f32>, // core radius, upward drift, XZ drift, circulation
    counts: vec4<u32>, // rings, vertices per ring, padding
};

fn filament_hash_u32(value: u32) -> u32 {
    var h = value;
    h = (h ^ (h >> 16u)) * 0x7feb352du;
    h = (h ^ (h >> 15u)) * 0x846ca68bu;
    return h ^ (h >> 16u);
}

fn filament_random01(seed: u32) -> f32 {
    return f32(filament_hash_u32(seed) >> 8u) / 16777216.0;
}

fn filament_limit_velocity(v: vec3<f32>, maximum: f32) -> vec3<f32> {
    return v * min(1.0, maximum / max(length(v), 0.000001));
}

fn filament_segment_velocity(start: vec3<f32>, end: vec3<f32>, position: vec3<f32>) -> vec3<f32> {
    let g0 = start - position;
    let g1 = end - position;
    let d = dot(g0, g1);
    let r0 = dot(g0, g0);
    let r1 = dot(g1, g1);
    let a2 = filament_params.motion.x * filament_params.motion.x;
    let cross_g = cross(g1, g0);
    let numerator = (d - r0) * inverseSqrt(a2 + r0) + (d - r1) * inverseSqrt(a2 + r1);
    let segment = g1 - g0;
    let denominator = max(a2 * dot(segment, segment) + dot(cross_g, cross_g), 0.00000001);
    return cross_g * (-numerator / denominator);
}

fn filament_velocity(position: vec3<f32>) -> vec3<f32> {
    var velocity = vec3<f32>(0.0);
    let lifetime = filament_params.step.w;
    for (var ring = 0u; ring < filament_params.counts.x; ring += 1u) {
        let base = ring * filament_params.counts.y;
        let age = filament_vertices[base].w;
        let weight = smoothstep(0.0, lifetime * 0.05, age)
            * (1.0 - smoothstep(lifetime * (5.0 / 6.0), lifetime, age));
        for (var vertex = 0u; vertex < filament_params.counts.y; vertex += 1u) {
            let start = filament_vertices[base + vertex].xyz;
            let end = filament_vertices[base + (vertex + 1u) % filament_params.counts.y].xyz;
            velocity += filament_segment_velocity(start, end, position) * weight;
        }
    }
    return filament_limit_velocity(velocity * filament_params.motion.w * filament_params.step.z, 0.6);
}
