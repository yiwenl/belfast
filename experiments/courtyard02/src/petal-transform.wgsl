struct Uniforms {
    view_projection: mat4x4<f32>,
    options: vec4<f32>, // sRGB attachment, petal size, reserved
};
@group(0) @binding(0) var<uniform> scene: Uniforms;

@group(0) @binding(1) var<storage, read> petals: array<PetalInstance>;

fn rotate_by_quaternion(v: vec3<f32>, q: vec4<f32>) -> vec3<f32> {
    let t = 2.0 * cross(q.xyz, v);
    return v + q.w * t + cross(q.xyz, t);
}

fn petal_world_position(position: vec3<f32>, petal: PetalInstance) -> vec3<f32> {
    return petal.position.xyz + rotate_by_quaternion(position * scene.options.y * petal.position.w, petal.rotation);
}
