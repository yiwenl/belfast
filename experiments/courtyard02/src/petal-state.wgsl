struct PetalInstance {
    position: vec4<f32>, // world position, lifecycle scale
    rotation: vec4<f32>, // unit quaternion, xyzw
    velocity: vec4<f32>, // velocity, age in seconds
    origin: vec4<f32>, // flower source, lifetime in seconds
    spin: vec4<f32>, // rotation axis, angular speed
};
