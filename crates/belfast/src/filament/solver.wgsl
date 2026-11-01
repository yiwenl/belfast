@group(0) @binding(0) var<uniform> filament_params: FilamentParams;
@group(0) @binding(1) var<storage, read> filament_vertices: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> filaments_out: array<vec4<f32>>;

@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= filament_params.counts.x * filament_params.counts.y) { return; }
    let ring = id.x / filament_params.counts.y;
    let previous = filament_vertices[id.x];
    let dt = filament_params.step.x;
    let lifetime = filament_params.step.w;
    let age = previous.w + dt;
    if (age >= lifetime) {
        let vertex = id.x % filament_params.counts.y;
        let cycle = u32(floor(filament_params.step.y / lifetime));
        let seed = filament_hash_u32(ring + cycle * 7919u);
        let angle = 6.28318530718 * f32(vertex) / f32(filament_params.counts.y);
        let radius = filament_params.origin_radius.w * (0.85 + 0.3 * filament_random01(seed + 1u));
        let offset = vec3<f32>((filament_random01(seed + 2u) - 0.5) * radius, 0.0,
            (filament_random01(seed + 3u) - 0.5) * radius * 0.2);
        let variation = (filament_random01(seed + vertex * 31u + 4u) - 0.5) * radius * 0.08;
        let position = filament_params.origin_radius.xyz + offset
            + vec3<f32>(cos(angle) * radius, variation, sin(angle) * radius);
        filaments_out[id.x] = vec4<f32>(position, age % lifetime);
        return;
    }
    // All vertices read the same previous state and share a coherent heading per ring.
    let phase = 6.28318530718 * filament_random01(ring + 97u);
    let time = filament_params.step.y + dt * 0.5;
    let heading = phase + 0.65 * sin(time * 0.45 + phase);
    let drift_speed = filament_params.motion.z * (0.7 + 0.3 * filament_random01(ring + 193u));
    let wind = vec3<f32>(cos(heading) * drift_speed, filament_params.motion.y, sin(heading) * drift_speed);
    let v0 = filament_velocity(previous.xyz) + wind;
    let midpoint = previous.xyz + v0 * dt * 0.5;
    let velocity = filament_limit_velocity(filament_velocity(midpoint) + wind, 0.8);
    filaments_out[id.x] = vec4<f32>(previous.xyz + velocity * dt, age);
}
