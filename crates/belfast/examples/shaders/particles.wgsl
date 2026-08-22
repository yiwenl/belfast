struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_right: vec4<f32>,
    camera_up: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) local: vec2<f32>,
};

@vertex
fn vs_main(
    @location(0) local: vec3<f32>,
    @location(1) instance_pos: vec3<f32>,
    @location(2) instance_color: vec3<f32>,
    @location(3) instance_size: f32,
) -> VertexOutput {
    var output: VertexOutput;
    let world_pos =
        instance_pos +
        camera.camera_right.xyz * local.x * instance_size +
        camera.camera_up.xyz * local.y * instance_size;
    output.position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    output.color = instance_color;
    output.local = local.xy;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    if (length(input.local * 2.0) > 1.0) {
        discard;
    }
    return vec4<f32>(input.color, 1.0);
}
