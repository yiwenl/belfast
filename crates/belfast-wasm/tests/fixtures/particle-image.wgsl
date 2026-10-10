@group(0) @binding(0) var<uniform> view_proj: mat4x4<f32>;
@group(1) @binding(0) var shadow_map: texture_depth_2d;
@group(1) @binding(1) var shadow_sampler: sampler_comparison;
@group(2) @binding(0) var image: texture_2d<f32>;
@group(2) @binding(1) var image_sampler: sampler;

struct Output {
    @builtin(position) position: vec4f,
    @location(0) color: vec3f,
};

@vertex
fn vs_main(@location(0) position: vec3f, @location(3) uv: vec2f) -> Output {
    return Output(view_proj * vec4f(position, 1.0), textureSampleLevel(image, image_sampler, uv, 0.0).rgb);
}

@fragment
fn fs_main(input: Output) -> @location(0) vec4f {
    let shade = textureSampleCompareLevel(shadow_map, shadow_sampler, vec2f(0.5), 0.5);
    return vec4f(input.color * shade, 1.0);
}
