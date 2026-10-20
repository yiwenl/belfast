struct SimParams {
    wind_friction: vec4<f32>,
    noise: vec4<f32>, // spatial scale, strength, time speed, reserved
    lifecycle: vec4<f32>, // hold fraction, scale-in fraction, scale-out fraction, reserved
    delta_seconds: f32,
    simulation_time: f32,
    count: u32,
    padding: u32,
}
@group(0) @binding(0) var<uniform> params: SimParams;
@group(0) @binding(1) var<storage, read> petals_in: array<Petal>;
@group(0) @binding(2) var<storage, read_write> petals_out: array<Petal>;

@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= params.count { return; }
    var p = petals_in[id.x];
    let dt = params.delta_seconds;
    if dt <= 0. { petals_out[id.x] = p; return; }
    p.position_life.w -= dt;
    if p.position_life.w <= 0. {
        p.position_life = p.origin_lifetime;
        p.velocity_angle = vec4(0., 0., 0., p.appearance.y);
        p.appearance.z = 0.;
    } else {
        let lifetime = p.origin_lifetime.w;
        let age = clamp(1. - p.position_life.w / lifetime, 0., 1.);
        let scale_in = smoothstep(0., params.lifecycle.y, age);
        let scale_out = 1. - smoothstep(1. - params.lifecycle.z, 1., age);
        p.appearance.z = scale_in * scale_out;

        // Count only the part of a crossing frame after the 20% hold has ended.
        let active_dt = clamp(lifetime * (1. - params.lifecycle.x) - p.position_life.w, 0., dt);
        if active_dt <= 0. {
            p.position_life = vec4(p.origin_lifetime.xyz, p.position_life.w);
            p.velocity_angle = vec4(0., 0., 0., p.appearance.y);
        } else {
            var position = p.position_life.xyz;
            var velocity = p.velocity_angle.xyz;
            // Small Euler steps retain the requested update order at any display FPS.
            let steps = max(1u, u32(ceil(active_dt * 120.)));
            let h = active_dt / f32(steps);
            let friction = pow(params.wind_friction.w, h * 60.);
            for (var step = 0u; step < steps; step += 1u) {
                let time = params.simulation_time + dt - active_dt + f32(step) * h;
                let offset = vec3(0.73, 0.41, -0.57) * time * params.noise.z;
                let flow = curl_noise(position * params.noise.x + offset);
                let force = params.wind_friction.xyz + flow * params.noise.y;
                velocity += force * h;
                // Apply the fixed speed to displacement, never compound it into velocity.
                position += velocity * h * p.appearance.w;
                velocity *= friction;
            }
            p.position_life = vec4(position, p.position_life.w);
            p.velocity_angle = vec4(velocity,
                fract((p.velocity_angle.w + p.axis_speed.w * active_dt) / 6.28318530718) * 6.28318530718);
        }
    }
    petals_out[id.x] = p;
}
