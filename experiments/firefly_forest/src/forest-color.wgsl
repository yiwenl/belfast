@group(0) @binding(0) var forest: texture_2d<f32>;

@vertex fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let corners = array<vec2<f32>, 3>(vec2<f32>(-1, -1), vec2<f32>(3, -1), vec2<f32>(-1, 3));
    return vec4<f32>(corners[index], 0.0, 1.0);
}

@fragment fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    // Same-size, same-format copy: no filtering, exposure, or tone mapping.
    return textureLoad(forest, vec2<i32>(position.xy), 0);
}
