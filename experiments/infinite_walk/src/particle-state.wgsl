struct Particle {
    position: vec4<f32>, // world XYZ, visible diameter
    fall_velocity: vec4<f32>, // gravity-only velocity XYZ, age; filament velocity is never stored
    color: u32,
    life: f32,
    seed: u32,
    force_scale: f32,
}

struct Motion {
    step: vec4<f32>, // dt, size, upward velocity, fall drag
    life: vec4<f32>, // lifetime, lifetime variation, ground Y, outside decay multiplier
    counts: vec2<u32>, // active particles, previous active particles
    force_multiplier: f32,
    gravity: f32, // downward acceleration in world units per second squared
    fluid_origin: vec4<f32>, // light-centered grid world XYZ, brush/depth radius
    fluid: vec4<f32>, // enabled, world grid width, noise phase, grid size
}

struct SourcePoint {
    position: vec3<f32>,
    color: u32,
}
