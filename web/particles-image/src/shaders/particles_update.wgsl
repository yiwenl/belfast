struct Particle {
    pos_size: vec4<f32>,
    velocity: vec4<f32>,
    color: vec4<f32>,
};

struct SimParams {
    time: f32,
    dt: f32,
    max_radius: f32,
    count: u32,
    noise_scale: f32,
    force_scale: f32,
    damping: f32,
    center_force: f32,
};

@group(0) @binding(0) var<uniform> params: SimParams;
@group(0) @binding(1) var<storage, read_write> particles: array<Particle>;

fn mod289_v4(x: vec4<f32>) -> vec4<f32> {
    return x - floor(x * (1.0 / 289.0)) * 289.0;
}

fn perm(x: vec4<f32>) -> vec4<f32> {
    return mod289_v4(((x * 34.0) + 1.0) * x);
}

fn noise(p: vec3<f32>) -> f32 {
    let a = floor(p);
    var d = p - a;
    d = d * d * (3.0 - 2.0 * d);

    let b = vec4(a.x, a.x, a.y, a.y) + vec4(0.0, 1.0, 0.0, 1.0);
    let k1 = perm(vec4(b.x, b.y, b.x, b.y));
    let k2 = perm(vec4(k1.x, k1.y, k1.x, k1.y) + vec4(b.z, b.z, b.w, b.w));

    let c = k2 + vec4(a.z, a.z, a.z, a.z);
    let k3 = perm(c);
    let k4 = perm(c + vec4(1.0));

    let o1 = fract(k3 * (1.0 / 41.0));
    let o2 = fract(k4 * (1.0 / 41.0));

    let o3 = o2 * d.z + o1 * (1.0 - d.z);
    let o4 = vec2(o3.y, o3.w) * d.x + vec2(o3.x, o3.z) * (1.0 - d.x);

    return o4.y * d.y + o4.x * (1.0 - d.y);
}

fn snoise(p: vec3<f32>) -> f32 {
    return noise(p) * 2.0 - 1.0;
}

fn snoise_vec3(x: vec3<f32>) -> vec3<f32> {
    let s = snoise(x);
    let s1 = snoise(vec3(x.y - 19.1, x.z + 33.4, x.x + 47.2));
    let s2 = snoise(vec3(x.z + 74.2, x.x - 124.5, x.y + 99.4));
    return vec3(s, s1, s2);
}

fn curl_noise(p: vec3<f32>) -> vec3<f32> {
    let e = 0.1;
    let dx = vec3(e, 0.0, 0.0);
    let dy = vec3(0.0, e, 0.0);
    let dz = vec3(0.0, 0.0, e);

    let p_x0 = snoise_vec3(p - dx);
    let p_x1 = snoise_vec3(p + dx);
    let p_y0 = snoise_vec3(p - dy);
    let p_y1 = snoise_vec3(p + dy);
    let p_z0 = snoise_vec3(p - dz);
    let p_z1 = snoise_vec3(p + dz);

    let x = p_y1.z - p_y0.z - p_z1.y + p_z0.y;
    let y = p_z1.x - p_z0.x - p_x1.z + p_x0.z;
    let z = p_x1.y - p_x0.y - p_y1.x + p_y0.x;

    let divisor = 1.0 / (2.0 * e);
    return normalize(vec3(x, y, z) * divisor);
}

@compute @workgroup_size(256)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= params.count) {
        return;
    }

    var particle = particles[index];
    var pos = particle.pos_size.xyz;
    var vel = particle.velocity.xyz;

    let time_offset = vec3<f32>(
        params.time * 0.16,
        params.time * 0.11,
        params.time * 0.13,
    );
    let force_mul = mix(0.5, 2.0, particle.velocity.w) * mix(0.5, 2.0, particle.color.w) * 1.5;
    var posOffset = snoise(pos + params.time * 0.5);
    posOffset = posOffset * posOffset;
    posOffset = mix(0.5, 1.0, posOffset);
    var force = curl_noise((pos * posOffset + time_offset) * params.noise_scale)
        * params.force_scale
        * force_mul;

    let dist = length(pos);
    if (dist > params.max_radius && dist > 0.0) {
        force += -normalize(pos) * (dist - params.max_radius) * params.center_force;
    }

    vel = (vel + force * params.dt) * params.damping;
    pos = pos + vel * params.dt;

    particle.pos_size = vec4<f32>(pos, particle.pos_size.w);
    particle.velocity = vec4<f32>(vel, particle.velocity.w);
    particles[index] = particle;
}
