@group(0) @binding(0) var<uniform> fog: FogParams;
@group(0) @binding(1) var merged_depth: texture_2d<f32>;
@group(0) @binding(2) var noise: texture_3d<f32>;
@group(0) @binding(3) var noise_sampler: sampler;
@group(0) @binding(4) var fallback_depth: texture_depth_2d;

struct Vertex {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};
struct FogOutput {
    @location(0) scattering: vec4<f32>, // integrated light, transmittance
    @location(1) distance: f32,
};

@vertex fn vs_main(@builtin(vertex_index) index: u32) -> Vertex {
    let p = array<vec2<f32>, 3>(vec2(-1., -1.), vec2(3., -1.), vec2(-1., 3.))[index];
    return Vertex(vec4(p, 0., 1.), vec2(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5));
}

fn density_at(p: vec3<f32>) -> f32 {
    let wind = vec3(0.08, 0.015, 0.035) * fog.eye_time.w;
    let moving = (p - wind) * fog.controls.y;
    let broad = textureSampleLevel(noise, noise_sampler, moving * 0.035, 0.).r;
    let detail = textureSampleLevel(noise, noise_sampler, moving * 0.08 + vec3(0.31), 0.).r;
    let billows = smoothstep(0.23, 0.77, broad * 0.75 + detail * 0.25);
    let height = exp(-max(p.y + 0.5, 0.) * 0.18);
    return fog.controls.x * mix(0.2, 1.6, billows) * height;
}

fn integrate(uv: vec2<f32>, depth: f32) -> FogOutput {
    let far_point = fog.inverse_camera * vec4(uv.x * 2. - 1., 1. - uv.y * 2., 1., 1.);
    let ray = normalize(far_point.xyz / far_point.w - fog.eye_time.xyz);
    var distance = fog.light_position.w;
    if depth < 1. {
        let surface = fog.inverse_camera * vec4(uv.x * 2. - 1., 1. - uv.y * 2., depth, 1.);
        distance = min(distance, length(surface.xyz / surface.w - fog.eye_time.xyz));
    }
    let step_size = distance / 32.;
    var transmittance = 1.;
    var light = vec3(0.);
    let g = fog.light_color.w;
    // The opening shines down toward the foreground; no firefly-light sampling.
    let light_axis = normalize(vec3(0., -2.6, 9.));
    for (var i = 0u; i < 32u; i += 1u) {
        let t = (f32(i) + 0.5) * step_size;
        let p = fog.eye_time.xyz + ray * t;
        let density = density_at(p) * smoothstep(0.3, 1.2, t);
        let step_transmittance = exp(-density * step_size);
        let delta = fog.light_position.xyz - p;
        let d2 = max(dot(delta, delta), 0.01);
        let to_light = delta * inverseSqrt(d2);
        let cosine = dot(ray, to_light);
        let phase_base = max(0.05, 1. + g * g - 2. * g * cosine);
        let phase = (1. - g * g) / (phase_base * sqrt(phase_base));
        let cone = smoothstep(0.57, 0.87, dot(-to_light, light_axis));
        let illumination = fog.light_color.rgb * fog.controls.z * phase * cone / (1. + d2 * 0.06);
        let source = vec3(0.035, 0.07, 0.10) + illumination;
        light += transmittance * (1. - step_transmittance) * source;
        transmittance *= step_transmittance;
        if transmittance < 0.01 { break; }
    }
    return FogOutput(vec4(light, transmittance), distance);
}

@fragment fn fs_main(in: Vertex) -> FogOutput {
    let size = textureDimensions(merged_depth);
    let pixel = clamp(vec2<i32>(in.uv * vec2<f32>(size)), vec2(0), vec2<i32>(size) - 1);
    return integrate(in.uv, textureLoad(merged_depth, pixel, 0).r);
}
@fragment fn fs_depth(in: Vertex) -> FogOutput {
    let size = textureDimensions(fallback_depth);
    let pixel = clamp(vec2<i32>(in.uv * vec2<f32>(size)), vec2(0), vec2<i32>(size) - 1);
    return integrate(in.uv, textureLoad(fallback_depth, pixel, 0));
}
