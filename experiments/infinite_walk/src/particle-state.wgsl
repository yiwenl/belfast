struct Particle {
    position: vec4<f32>, // world XYZ, visible diameter
    velocity: vec4<f32>, // velocity XYZ, age
    color: u32,
    life: f32,
    seed: u32,
    force_scale: f32,
}

struct Motion {
    step: vec4<f32>, // dt, size, upward velocity, drag
    life: vec4<f32>, // lifetime, lifetime variation, ground Y, outside decay multiplier
    counts: vec4<u32>, // active particles, previous active particles, padding
}

struct SourcePoint {
    position: vec3<f32>,
    color: u32,
}

fn inside_spotlight(position: vec3<f32>) -> bool {
    let offset = position - camera.light.xyz;
    let distance = length(offset);
    return camera.light.w > 0.0 && distance < camera.light_shape.x
        && -offset.y > camera.light_shape.z * max(distance, 0.0001);
}
