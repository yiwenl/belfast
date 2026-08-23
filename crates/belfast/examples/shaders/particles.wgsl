struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_right: vec4<f32>,
    camera_up: vec4<f32>,
    light_view_proj: mat4x4<f32>,
    shadow_params: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var shadow_map: texture_depth_2d;
@group(1) @binding(1) var shadow_sampler: sampler_comparison;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) local: vec2<f32>,
    @location(2) shadow_coord: vec4<f32>,
};

@vertex
fn vs_main(
    @location(0) local: vec3<f32>,
    @location(1) instance_pos_size: vec4<f32>,
    @location(2) instance_color: vec4<f32>,
) -> VertexOutput {
    var output: VertexOutput;
    let world_pos =
        instance_pos_size.xyz +
        camera.camera_right.xyz * local.x * instance_pos_size.w +
        camera.camera_up.xyz * local.y * instance_pos_size.w;
    output.position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    output.color = instance_color.xyz;
    output.local = local.xy;
    output.shadow_coord = camera.light_view_proj * vec4<f32>(world_pos, 1.0);
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    if (length(input.local * 2.0) > 1.0) {
        discard;
    }
    let visibility = sampleShadowPcf3x3(
        shadow_map,
        shadow_sampler,
        input.shadow_coord,
        camera.shadow_params.y,
        camera.shadow_params.z,
    );
    let shade = mix(1.0 - camera.shadow_params.x, 1.0, visibility);
    return vec4<f32>(input.color * shade, 1.0);
}
