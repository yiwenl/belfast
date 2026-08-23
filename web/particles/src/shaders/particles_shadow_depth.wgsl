struct LightUniform {
    view_proj: mat4x4<f32>,
    camera_right: vec4<f32>,
    camera_up: vec4<f32>,
};

@group(0) @binding(0) var<uniform> light: LightUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
};

@vertex
fn vs_main(
    @location(0) local: vec3<f32>,
    @location(1) instance_pos_size: vec4<f32>,
) -> VertexOutput {
    var output: VertexOutput;
    let world_pos =
        instance_pos_size.xyz +
        light.camera_right.xyz * local.x * instance_pos_size.w +
        light.camera_up.xyz * local.y * instance_pos_size.w;
    output.position = light.view_proj * vec4<f32>(world_pos, 1.0);
    output.local = local.xy;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) {
    if (length(input.local * 2.0) > 1.0) {
        discard;
    }
}
