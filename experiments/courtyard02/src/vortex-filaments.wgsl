@group(0) @binding(2) var<storage, read_write> filaments_out: array<vec4<f32>>;

@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= flow.counts.y * flow.counts.z) { return; }
    let ring = id.x / flow.counts.z;
    let previous = filaments_in[id.x];
    let age = previous.w + flow.step.x;
    if (age >= FILAMENT_LIFETIME) {
        let vertex = id.x % flow.counts.z;
        let cycle = u32(floor(flow.step.y / FILAMENT_LIFETIME));
        let seed = hash_u32(ring + cycle * 7919u);
        let angle = 6.28318530718 * f32(vertex) / f32(flow.counts.z);
        let radius = flow.origin_radius.w * (0.85 + 0.3 * random01(seed + 1u));
        let offset = vec3<f32>((random01(seed + 2u) - 0.5) * radius, 0.0,
            (random01(seed + 3u) - 0.5) * radius * 0.2);
        let variation = (random01(seed + vertex * 31u + 4u) - 0.5) * radius * 0.08;
        let position = flow.origin_radius.xyz + offset
            + vec3<f32>(cos(angle) * radius, variation, sin(angle) * radius);
        filaments_out[id.x] = vec4<f32>(position, age - FILAMENT_LIFETIME);
        return;
    }
    // Advect all vertices from the same previous state; no in-place neighbor reads.
    // A shared heading translates each ring coherently instead of shaking individual vertices.
    let phase = 6.28318530718 * random01(ring + 97u);
    let time = flow.step.y + flow.step.x * 0.5;
    let heading = phase + 0.65 * sin(time * 0.45 + phase);
    let drift_speed = flow.filament.z * (0.7 + 0.3 * random01(ring + 193u));
    let wind = vec3<f32>(cos(heading) * drift_speed, flow.filament.y, sin(heading) * drift_speed);
    let v0 = vortex_velocity(previous.xyz) + wind;
    let midpoint = previous.xyz + v0 * flow.step.x * 0.5;
    let velocity = limit_velocity(vortex_velocity(midpoint) + wind, 0.8);
    filaments_out[id.x] = vec4<f32>(previous.xyz + velocity * flow.step.x, age);
}
