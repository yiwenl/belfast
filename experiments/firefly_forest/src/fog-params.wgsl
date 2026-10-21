struct FogParams {
    inverse_camera: mat4x4<f32>,
    eye_time: vec4<f32>,
    controls: vec4<f32>, // density, noise frequency, light intensity, enabled
    light_position: vec4<f32>, // xyz, maximum ray distance
    light_color: vec4<f32>, // rgb, anisotropy
};
