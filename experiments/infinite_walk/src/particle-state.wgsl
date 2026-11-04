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
    counts: vec2<u32>, // active particles, previous active particles
    force_multiplier: f32,
    padding: u32,
}

struct SourcePoint {
    position: vec3<f32>,
    color: u32,
}
