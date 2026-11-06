@group(0) @binding(7) var<storage, read> pointer_fluid: array<vec4<f32>>;

fn pointer_fluid_cell(p: vec2<i32>) -> vec2<f32> {
    let size = i32(motion.fluid.w);
    let cell = clamp(p, vec2(0), vec2(size - 1));
    return pointer_fluid[u32(cell.y * size + cell.x)].xy;
}

fn pointer_fluid_sample(uv: vec2<f32>) -> vec2<f32> {
    let p = uv * motion.fluid.w - 0.5;
    let cell = vec2<i32>(floor(p));
    let blend = fract(p);
    return mix(
        mix(pointer_fluid_cell(cell), pointer_fluid_cell(cell + vec2(1, 0)), blend.x),
        mix(pointer_fluid_cell(cell + vec2(0, 1)), pointer_fluid_cell(cell + vec2(1, 1)), blend.x),
        blend.y,
    );
}

fn pointer_noise_hash(cell: vec3<i32>) -> f32 {
    let c = bitcast<vec3<u32>>(cell) & vec3(15u);
    return filament_random01(c.x + c.y * 16u + c.z * 256u);
}

fn pointer_noise(p: vec3<f32>) -> f32 {
    let cell = vec3<i32>(floor(p));
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let lower = mix(
        mix(pointer_noise_hash(cell), pointer_noise_hash(cell + vec3(1, 0, 0)), u.x),
        mix(pointer_noise_hash(cell + vec3(0, 1, 0)), pointer_noise_hash(cell + vec3(1, 1, 0)), u.x),
        u.y,
    );
    let upper = mix(
        mix(pointer_noise_hash(cell + vec3(0, 0, 1)), pointer_noise_hash(cell + vec3(1, 0, 1)), u.x),
        mix(pointer_noise_hash(cell + vec3(0, 1, 1)), pointer_noise_hash(cell + vec3(1, 1, 1)), u.x),
        u.y,
    );
    return mix(lower, upper, u.z) * 2.0 - 1.0;
}

fn pointer_velocity(position: vec3<f32>) -> vec3<f32> {
    if motion.fluid.x <= 0.0 { return vec3(0.0); }
    let offset = position - motion.fluid_origin.xyz;
    let radius = motion.fluid_origin.w;
    let uv = offset.xy / motion.fluid.y + 0.5;
    if abs(offset.z) >= radius || any(uv < vec2(0.0)) || any(uv > vec2(1.0)) { return vec3(0.0); }
    // Sample the persistent field, including trails left behind by the moving pointer.
    let weight = 1.0 - smoothstep(0.0, radius, abs(offset.z));
    let flow = pointer_fluid_sample(uv) * motion.fluid.y;
    let z = pointer_noise(offset / radius * 2.0 + vec3(0.0, 0.0, motion.fluid.z)) * length(flow) * 0.35;
    return filament_limit_velocity(vec3(flow, z), 10.0) * weight;
}
