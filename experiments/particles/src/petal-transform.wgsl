@group(1) @binding(0) var<storage, read> petals: array<Petal>;
@group(1) @binding(1) var<uniform> petal_render: vec4<f32>; // global scale, reserved

fn rotate_petal(v: vec3<f32>, axis: vec3<f32>, angle: f32) -> vec3<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return v * c + cross(axis, v) * s + axis * dot(axis, v) * (1. - c);
}

fn petal_world_position(position: vec3<f32>, petal: Petal) -> vec3<f32> {
    let scale = petal.appearance.x * petal.appearance.z * petal_render.x;
    return petal.position_life.xyz + rotate_petal(position * scale, petal.axis_speed.xyz, petal.velocity_angle.w);
}
