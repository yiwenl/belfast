@group(0) @binding(0) var<uniform> viewport: vec4<f32>;
@group(0) @binding(1) var<storage, read> field: array<vec4<f32>>;

struct Output {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) map: u32,
}

@vertex fn vs_main(@builtin(vertex_index) vertex: u32, @builtin(instance_index) instance: u32) -> Output {
    let corners = array<vec2<f32>, 6>(vec2(0,0), vec2(1,0), vec2(0,1), vec2(0,1), vec2(1,0), vec2(1,1));
    let uv = corners[vertex];
    let css = viewport.xy / viewport.z;
    // Match the CSS label layout; keep the overlays in CSS pixels at every DPR.
    let size = max(1.0, min(160.0, min((css.x - 44.0) * 0.5, css.y - 56.0)));
    let pixel = vec2<f32>(16.0 + f32(instance) * (size + 12.0), 16.0) + uv * size;
    var out: Output;
    out.position = vec4<f32>(pixel / css * 2.0 - 1.0, 0.0, 1.0);
    out.uv = uv;
    out.map = instance;
    return out;
}

@fragment fn fs_main(input: Output) -> @location(0) vec4<f32> {
    let n = u32(viewport.w);
    let cell = min(vec2<u32>(input.uv * viewport.w), vec2<u32>(n - 1u));
    let value = field[cell.y * n + cell.x];
    var color = vec3<f32>(clamp(value.w / 2.0, 0.0, 1.0));
    if input.map == 0u {
        // Map signed XY velocity (-1..1 UV/s) to RG; zero velocity is 0.5.
        color = vec3<f32>(clamp(value.xy * 0.5 + 0.5, vec2<f32>(0.0), vec2<f32>(1.0)), 0.0);
    }
    // The canvas uses an sRGB attachment, so convert the debug display colors to linear.
    return vec4<f32>(pow(color, vec3<f32>(2.2)), 1.0);
}
