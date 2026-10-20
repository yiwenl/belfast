struct Uniforms {
    view_projection: mat4x4<f32>,
    base_factor: vec4<f32>,
    options: vec4<f32>, // normal strength, sRGB attachment, reserved
    point_position_intensity: vec4<f32>,
    point_color: vec4<f32>, // linear RGB
}
@group(0) @binding(0) var<uniform> scene: Uniforms;
@group(0) @binding(1) var base_color: texture_2d<f32>;
@group(0) @binding(2) var normal_map: texture_2d<f32>;
@group(0) @binding(3) var material_sampler: sampler;

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}
@vertex fn vs_main(@location(0) p: vec3<f32>, @location(1) n: vec3<f32>, @location(2) uv: vec2<f32>) -> VertexOut {
    var out: VertexOut;
    out.clip = scene.view_projection * vec4(p, 1.);
    out.position = p;
    out.normal = n;
    out.uv = uv;
    return out;
}

fn linear_to_srgb(rgb: vec3<f32>) -> vec3<f32> {
    return select(1.055 * pow(max(rgb, vec3(0.)), vec3(1. / 2.4)) - 0.055,
        12.92 * rgb, rgb <= vec3(0.0031308));
}

@fragment fn fs_main(in: VertexOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    let n = select(-normalize(in.normal), normalize(in.normal), front);
    let dp1 = dpdx(in.position);
    let dp2 = dpdy(in.position);
    let duv1 = dpdx(in.uv);
    let duv2 = dpdy(in.uv);
    let p2 = cross(dp2, n);
    let p1 = cross(n, dp1);
    let t = p2 * duv1.x + p1 * duv2.x;
    let b = p2 * duv1.y + p1 * duv2.y;
    let frame_scale = inverseSqrt(max(max(dot(t, t), dot(b, b)), 1e-16));
    let sample_normal = textureSample(normal_map, material_sampler, in.uv).xyz * 2. - 1.;
    let normal = normalize(t * frame_scale * sample_normal.x * scene.options.x
        + b * frame_scale * sample_normal.y * scene.options.x + n * max(sample_normal.z, 0.001));
    let albedo = textureSample(base_color, material_sampler, in.uv).rgb * scene.base_factor.rgb;
    // A small ambient term keeps the shadowed sides readable without directional fill.
    let ambient = vec3(0.05);
    let to_point = scene.point_position_intensity.xyz - in.position;
    let distance_squared = dot(to_point, to_point);
    let point_direction = to_point * inverseSqrt(max(distance_squared, 1e-6));
    // Softened inverse-square attenuation stays finite at the light position.
    let attenuation = scene.point_position_intensity.w / (1. + distance_squared);
    let visibility = point_visibility(in.position, n, point_direction);
    let point = scene.point_color.rgb * attenuation * max(dot(normal, point_direction), 0.);
    var color = albedo * (ambient + point);
    color *= mix(0.0, 1.0, visibility);

    if scene.options.y < 0.5 { color = linear_to_srgb(color); }
    return vec4(color, 1.);
}
