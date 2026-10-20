@group(0) @binding(0) var forest: texture_2d<f32>;
override srgb_attachment: bool = false;
struct Grade {
    params: vec4<f32>, // strength, brightness
    shadow: vec4<f32>,
    highlight: vec4<f32>,
    foliage: vec4<f32>,
};
@group(0) @binding(1) var<uniform> grade: Grade;
@group(0) @binding(2) var light_map: texture_3d<f32>;
@group(0) @binding(3) var light_sampler: sampler;
@group(0) @binding(4) var<uniform> volume: LightVolumeParams;
@group(0) @binding(5) var<uniform> inverse_camera: mat4x4<f32>;
@group(0) @binding(6) var merged_depth: texture_2d<f32>;
@group(0) @binding(7) var fallback_depth: texture_depth_2d;

fn lighting(position: vec2<f32>, depth: f32) -> vec3<f32> {
    if volume.extent_strength.w <= 0. || depth >= 1. { return vec3(0.); }
    let screen = position / vec2<f32>(textureDimensions(forest));
    let homogeneous = inverse_camera * vec4(screen.x * 2. - 1., 1. - screen.y * 2., depth, 1.);
    let world = homogeneous.xyz / homogeneous.w;
    let uvw = (world - volume.min_radius.xyz) / volume.extent_strength.xyz;
    if any(uvw < vec3(0.)) || any(uvw > vec3(1.)) { return vec3(0.); }
    return textureSampleLevel(light_map, light_sampler, uvw.xzy, 0.).rgb * volume.extent_strength.w;
}

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

fn shade_forest(position: vec4<f32>, depth: f32) -> vec4<f32> {
    let forest_color = textureLoad(forest, vec2<i32>(position.xy), 0);
    var rgb = max(forest_color.rgb, vec3(0.));
    // Grade display RGB consistently on HDR/unorm and hardware-sRGB targets.
    if srgb_attachment {
        rgb = select(rgb * 12.92, 1.055 * pow(rgb, vec3(1. / 2.4)) - 0.055, rgb > vec3(0.0031308));
    }
    let albedo = clamp(rgb, vec3(0.025), vec3(1.));
    rgb = mix(rgb, night_grade(rgb), grade.params.x) * grade.params.y;
    // shortcut: the nearest opaque-enough splat supplies one surface, not translucent layers or normals.
    rgb += lighting(position.xy, depth) * albedo;
    if srgb_attachment {
        rgb = select(rgb / 12.92, pow((rgb + 0.055) / 1.055, vec3(2.4)), rgb > vec3(0.04045));
    }
    return vec4(rgb, forest_color.a);
}

@fragment fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    return shade_forest(position, textureLoad(merged_depth, vec2<i32>(position.xy), 0).r);
}

@fragment fn fs_depth(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    return shade_forest(position, textureLoad(fallback_depth, vec2<i32>(position.xy), 0));
}
