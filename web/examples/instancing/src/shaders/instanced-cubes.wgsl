struct SceneUniforms {
    viewProj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> scene: SceneUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) instancePosSize: vec4<f32>,
    @location(3) instanceColor: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    let scale = input.instancePosSize.w;
    let worldPos = input.instancePosSize.xyz + input.position * scale;
    output.position = scene.viewProj * vec4<f32>(worldPos, 1.0);
    output.normal = input.normal;
    output.color = input.instanceColor.xyz;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(input.normal);
    let lightDir = normalize(vec3<f32>(-0.45, -0.8, -0.35));
    let diffuse = max(dot(n, -lightDir), 0.0);
    let color = input.color * (0.22 + 0.78 * diffuse);
    return vec4<f32>(color, 1.0);
}
