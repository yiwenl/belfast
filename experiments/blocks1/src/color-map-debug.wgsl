@group(0) @binding(0) var<uniform> viewport: vec4<f32>;
@group(0) @binding(1) var color_map: texture_2d<f32>;
@group(0) @binding(2) var color_sampler: sampler;

struct Output {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex fn vs_main(@builtin(vertex_index) vertex: u32) -> Output {
    let corners = array<vec2<f32>, 6>(vec2(0,0), vec2(1,0), vec2(0,1), vec2(0,1), vec2(1,0), vec2(1,1));
    let uv = corners[vertex];
    let css = viewport.xy / viewport.z;
    let size = max(1.0, min(192.0, min(css.x, css.y) - 32.0));
    let pixel = vec2<f32>(16.0) + uv * size;
    var out: Output;
    out.position = vec4<f32>(pixel.x / css.x * 2.0 - 1.0, 1.0 - pixel.y / css.y * 2.0, 0.0, 1.0);
    out.uv = uv;
    return out;
}

@fragment fn fs_main(input: Output) -> @location(0) vec4<f32> {
    let color = textureSample(color_map, color_sampler, input.uv).rgb;
    // The palette stores sRGB bytes in an unorm texture; the canvas attachment encodes sRGB.
    let linear = select(pow((color + 0.055) / 1.055, vec3<f32>(2.4)), color / 12.92, color <= vec3<f32>(0.04045));
    return vec4<f32>(linear, 1.0);
}
