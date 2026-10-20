@group(0) @binding(0) var forest: texture_2d<f32>;
override srgb_attachment: bool = false;
struct Grade {
    params: vec4<f32>, // strength, brightness
    shadow: vec4<f32>,
    highlight: vec4<f32>,
    foliage: vec4<f32>,
};
@group(0) @binding(1) var<uniform> grade: Grade;

fn night_grade(rgb: vec3<f32>) -> vec3<f32> {
    let source = clamp(rgb, vec3(0.), vec3(1.));
    let luminance = dot(source, vec3(0.2126, 0.7152, 0.0722));
    // Source green dominance separates foliage from warmer bark and the path.
    let green = (source.g - max(source.r, source.b)) / max(luminance, 0.04);
    let foliage = smoothstep(0.02, 0.30, green);
    let moonlight = smoothstep(0.45, 1., luminance);
    let blue = mix(grade.shadow.rgb, grade.highlight.rgb, moonlight);
    let teal = grade.foliage.rgb;
    let palette = mix(blue, teal, foliage * smoothstep(0.04, 0.25, luminance));
    let tone = luminance / (0.42 + 0.58 * luminance);
    return mix(vec3(0.003, 0.009, 0.024), palette, tone);
}

@vertex fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let corners = array<vec2<f32>, 3>(vec2<f32>(-1, -1), vec2<f32>(3, -1), vec2<f32>(-1, 3));
    return vec4<f32>(corners[index], 0.0, 1.0);
}

@fragment fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let forest_color = textureLoad(forest, vec2<i32>(position.xy), 0);
    var rgb = max(forest_color.rgb, vec3(0.));
    // Grade display RGB consistently on HDR/unorm and hardware-sRGB targets.
    if srgb_attachment {
        rgb = select(rgb * 12.92, 1.055 * pow(rgb, vec3(1. / 2.4)) - 0.055, rgb > vec3(0.0031308));
    }
    rgb = mix(rgb, night_grade(rgb), grade.params.x) * grade.params.y;
    if srgb_attachment {
        rgb = select(rgb / 12.92, pow((rgb + 0.055) / 1.055, vec3(2.4)), rgb > vec3(0.04045));
    }
    return vec4(rgb, forest_color.a);
}
