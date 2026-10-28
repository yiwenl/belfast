struct Particle {
    position: vec4<f32>,
    velocity: vec4<f32>, // xyz velocity, w = fixed color-transition random
    origin: vec4<f32>,
    extra: vec4<f32>,
}
