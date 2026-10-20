struct Sky {
    rays: mat4x4<f32>,
    // mode (day=0, night=1, panorama=2), sRGB attachment, padding.
    options: vec4<f32>,
};
@group(0) @binding(0) var<uniform> sky: Sky;
@group(0) @binding(1) var panorama: texture_2d<f32>;
@group(0) @binding(2) var panorama_sampler: sampler;
struct Vertex {
    @builtin(position) position: vec4<f32>,
    @location(0) ndc: vec2<f32>,
};
@vertex fn vs_main(@builtin(vertex_index) index: u32) -> Vertex {
    let p = array<vec2<f32>, 3>(vec2<f32>(-1,-1), vec2<f32>(3,-1), vec2<f32>(-1,3));
    var out: Vertex;
    out.position = vec4<f32>(p[index], 1.0, 1.0);
    out.ndc = p[index];
    return out;
}
@fragment fn fs_main(in: Vertex) -> @location(0) vec4<f32> {
    let ray = normalize((sky.rays * vec4<f32>(in.ndc, 1.0, 1.0)).xyz);
    var horizon = vec3<f32>(0.86, 0.89, 0.93);
    var zenith = vec3<f32>(0.44, 0.65, 0.90);
    if (sky.options.x > 0.5) {
        horizon = vec3<f32>(0.075, 0.10, 0.18);
        zenith = vec3<f32>(0.008, 0.016, 0.045);
    }
    var rgb = mix(horizon, zenith, pow(clamp(ray.y, 0.0, 1.0), 0.6));
    if (sky.options.x > 1.5) {
        // Center is original +Z (render -Z); original +X points right, original -Y up.
        let uv = vec2<f32>(0.5 + atan2(ray.x, -ray.z) / 6.28318530718,
            0.5 - asin(clamp(ray.y, -1.0, 1.0)) / 3.14159265359);
        rgb = textureSampleLevel(panorama, panorama_sampler, uv, 0.0).rgb;
    }
    if (sky.options.y > 0.5) {
        rgb = select(rgb / 12.92, pow((rgb + 0.055) / 1.055, vec3<f32>(2.4)), rgb > vec3<f32>(0.04045));
    }
    return vec4<f32>(rgb, 1.0);
}
