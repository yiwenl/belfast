struct FlowParams {
    shadow_view_projection: mat4x4<f32>,
    step: vec4<f32>, // dt, time, petal upward acceleration, padding
    counts: vec4<u32>, // petals, padding
};
@group(0) @binding(0) var<uniform> flow: FlowParams;
@group(0) @binding(1) var<storage, read> filament_vertices: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read> petals_in: array<PetalInstance>;
@group(0) @binding(3) var<storage, read_write> petals_out: array<PetalInstance>;
@group(0) @binding(4) var<uniform> filament_params: FilamentParams;

fn quaternion_multiply(a: vec4<f32>, b: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(a.w * b.xyz + b.w * a.xyz + cross(a.xyz, b.xyz), a.w * b.w - dot(a.xyz, b.xyz));
}

fn boundary_distance(position: vec3<f32>) -> f32 {
    let clip = flow.shadow_view_projection * vec4<f32>(position, 1.0);
    let p = clip.xyz / clip.w;
    return min(min(1.0 - abs(p.x), 1.0 - abs(p.y)), 2.0 * min(p.z, 1.0 - p.z));
}

@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= flow.counts.x) { return; }
    var p = petals_in[id.x];
    let dt = flow.step.x;
    let age = p.velocity.w + dt;
    if (age >= p.origin.w || boundary_distance(p.position.xyz) <= 0.0) {
        let seed = filament_hash_u32(id.x ^ u32(flow.step.y * 4096.0));
        let jitter = (vec3<f32>(filament_random01(seed), filament_random01(seed + 1u), filament_random01(seed + 2u)) - 0.5) * 0.02;
        let z_offset = vec3<f32>(0.0, 0.0, filament_random01(seed + 3u) * -0.2);
        p.position = vec4<f32>(p.origin.xyz + jitter + z_offset, 0.0);
        p.velocity = vec4<f32>(0.0);
        petals_out[id.x] = p;
        return;
    }

    let field = filament_velocity(p.position.xyz + p.velocity.xyz * dt * 0.5);
    // Air drag follows the filament flow; constant +Y acceleration provides lift.
    let drag = 1.6;
    let target_velocity = field + vec3<f32>(0.0, flow.step.z / drag, 0.0);
    let velocity = filament_limit_velocity(mix(p.velocity.xyz, target_velocity, 1.0 - exp(-drag * dt)), 0.8);
    let position = p.position.xyz + (p.velocity.xyz + velocity) * (dt * 0.5);
    let fade_in = smoothstep(0.0, 0.3, age);
    let fade_out = 1.0 - smoothstep(p.origin.w * 0.8, p.origin.w, age);
    let edge_fade = smoothstep(0.0, 0.12, boundary_distance(position));
    p.position = vec4<f32>(position, fade_in * fade_out * edge_fade);
    p.velocity = vec4<f32>(velocity, age);

    let omega = p.spin.xyz * p.spin.w + cross(vec3<f32>(0.0, 1.0, 0.0), field) * 3.0;
    let speed = length(omega);
    let half_angle = speed * dt * 0.5;
    let delta = vec4<f32>(omega * (sin(half_angle) / max(speed, 0.000001)), cos(half_angle));
    p.rotation = normalize(quaternion_multiply(delta, p.rotation));
    petals_out[id.x] = p;
}
