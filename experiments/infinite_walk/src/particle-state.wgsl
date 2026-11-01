struct Particle {
    position: vec4<f32>, // world XYZ, visible diameter
    velocity: vec4<f32>, // velocity XYZ, age
    color: u32,
    life: f32,
    seed: u32,
    padding: u32,
}

struct Motion {
    step: vec4<f32>, // dt, size, upward velocity, drag
    life: vec4<f32>, // lifetime, lifetime variation, ground Y, padding
    counts: vec4<u32>, // particles, padding
}

struct SourcePoint {
    position: vec3<f32>,
    color: u32,
}
