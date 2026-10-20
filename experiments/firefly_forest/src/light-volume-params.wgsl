struct LightVolumeParams {
    min_radius: vec4<f32>, // world minimum XYZ, light radius
    extent_strength: vec4<f32>, // world extent XYZ, enabled strength
    dimensions_capacity: vec4<u32>, // texture X/Z/Y dimensions, particle capacity
};
