struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
}

@group(0) @binding(0) var image_texture: texture_2d<f32>;
@group(0) @binding(1) var image_sampler: sampler;

@vertex
fn vs_main(@location(0) position: vec2f, @location(1) uv: vec2f) -> VertexOutput {
    return VertexOutput(vec4f(position, 0.0, 1.0), uv);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4f {
    return textureSample(image_texture, image_sampler, input.uv);
}
