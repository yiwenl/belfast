struct Firefly {
    position_phase: vec4<f32>, // world XYZ, circular phase in [0, 1)
    velocity_frequency: vec4<f32>, // world velocity, intrinsic cycles per second
    flash: vec4<f32>, // elapsed flash seconds, intensity, captured duration, fixed height seed [0, 1]
    drag: vec4<f32>, // individual brightness progress, padding
};
