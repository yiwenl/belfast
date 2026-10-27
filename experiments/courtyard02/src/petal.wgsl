struct Uniforms {
    view_projection: mat4x4<f32>,
    options: vec4<f32>, // sRGB attachment, petal size, reserved
};
@group(0) @binding(0) var<uniform> scene: Uniforms;

struct PetalInstance {
    position: vec4<f32>,
    rotation: vec4<f32>, // Unit quaternion, xyzw.
};
@group(0) @binding(1) var<storage, read> petals: array<PetalInstance>;

fn rotate_by_quaternion(v: vec3<f32>, q: vec4<f32>) -> vec3<f32> {
    let t = 2.0 * cross(q.xyz, v);
    return v + q.w * t + cross(q.xyz, t);
}

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4<f32>,
};

@vertex fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(3) color: vec4<f32>,
    @builtin(instance_index) index: u32,
) -> VertexOut {
    let petal = petals[index];
    let world_position = petal.position.xyz + rotate_by_quaternion(position * scene.options.y, petal.rotation);
    var out: VertexOut;
    out.position = scene.view_projection * vec4<f32>(world_position, 1.0);
    out.normal = rotate_by_quaternion(normal, petal.rotation);
    out.color = color;
    return out;
}

@fragment fn fs_main(in: VertexOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    let normal = select(-normalize(in.normal), normalize(in.normal), front);
    let light = normalize(vec3<f32>(-0.4, 0.8, 0.6));
    let diffuse = max(dot(normal, light), 0.0);
    let transmitted = max(dot(-normal, light), 0.0);
    // Soft skylight and a little backlighting keep both sides of the thin petal readable.
    var color = in.color.rgb * (0.48 + 0.48 * diffuse + 0.18 * transmitted);
    if (scene.options.x < 0.5) {
        color = select(12.92 * color, 1.055 * pow(color, vec3<f32>(1.0 / 2.4)) - 0.055,
            color > vec3<f32>(0.0031308));
    }
    return vec4<f32>(color, 1.0);
}
