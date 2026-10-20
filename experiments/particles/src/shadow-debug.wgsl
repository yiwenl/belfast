@group(0) @binding(0) var shadow_depth: texture_depth_2d;
@group(0) @binding(1) var<uniform> options: vec4<f32>; // sRGB output, reserved

struct DebugVertex {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex fn vs_main(@builtin(vertex_index) index: u32) -> DebugVertex {
    let positions = array<vec2<f32>, 3>(vec2(-1., -1.), vec2(3., -1.), vec2(-1., 3.));
    let p = positions[index];
    var out: DebugVertex;
    out.position = vec4(p, 0., 1.);
    out.uv = vec2(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5);
    return out;
}

@fragment fn fs_main(in: DebugVertex) -> @location(0) vec4<f32> {
    let size = textureDimensions(shadow_depth);
    let pixel = clamp(vec2<i32>(in.uv * vec2<f32>(size)), vec2(0), vec2<i32>(size) - vec2(1));
    // Read raw depth, without the comparison sampler or shadow bias.
    let depth = textureLoad(shadow_depth, pixel, 0);
    var gray = depth;
    if options.x > 0.5 {
        gray = select(depth / 12.92, pow((depth + 0.055) / 1.055, 2.4), depth > 0.04045);
    }
    return vec4(vec3(gray), 1.);
}
