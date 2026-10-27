struct Uniforms {
    view_projection: mat4x4<f32>,
    model: mat4x4<f32>,
    options: vec4<f32>, // sRGB attachment, reserved
};
@group(0) @binding(0) var<uniform> scene: Uniforms;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4<f32>,
};

@vertex fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(3) color: vec4<f32>,
) -> VertexOut {
    var out: VertexOut;
    out.position = scene.view_projection * scene.model * vec4<f32>(position, 1.0);
    out.normal = normalize((scene.model * vec4<f32>(normal, 0.0)).xyz);
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
