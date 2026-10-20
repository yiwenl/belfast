struct Camera {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
};
override peak_brightness: f32 = 1.8;
@group(0) @binding(0) var<storage, read> particles: array<Firefly>;
@group(0) @binding(1) var<uniform> camera: Camera;
struct Vertex {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) brightness: f32,
    @location(2) color: vec3<f32>,
};
@vertex fn vs_main(@builtin(vertex_index) vertex: u32, @builtin(instance_index) index: u32) -> Vertex {
    let p = particles[index];
    if p.flash.y <= 0. {
        // All six vertices lie outside the clip volume (z > w), so an unlit
        // billboard produces no fragments or additive blending work.
        return Vertex(vec4(2., 2., 2., 1.), vec2(0.), 0., vec3(0.));
    }
    let corners = array<vec2<f32>, 6>(vec2(-1,-1),vec2(1,-1),vec2(1,1),vec2(-1,-1),vec2(1,1),vec2(-1,1));
    let uv = corners[vertex];
    let variation = fract(f32(index) * 0.61803398875);
    // Compute supplies a fixed-duration envelope independent of cycle speed.
    let brightness = peak_brightness * p.flash.y;
    let radius = mix(0.045, 0.07, variation);
    let center = camera.view * vec4(p.position_phase.xyz, 1.);
    var out: Vertex;
    out.position = camera.projection * (center + vec4(uv * radius, 0., 0.));
    out.uv = uv;
    out.brightness = brightness;
    out.color = mix(vec3(0.72, 1., 0.10), vec3(1., 0.87, 0.18), variation);
    return out;
}
fn shade(in: Vertex) -> vec4<f32> {
    let r2 = dot(in.uv, in.uv);
    if r2 > 1. { discard; }
    let core = exp(-r2 * 100.);
    let halo = exp(-r2 * 6.) * 0.18 * (1. - smoothstep(0.6, 1., r2));
    let rgb = (vec3(1., 1., 0.65) * core + in.color * halo) * in.brightness;
    // Additive emission; the surrounding forest supplies the background color.
    return vec4(rgb, 0.);
}

@group(1) @binding(0) var occlusion: texture_2d<f32>;
@fragment fn fs_main(in: Vertex) -> @location(0) vec4<f32> {
    return shade(in);
}
@fragment fn fs_occluded(in: Vertex) -> @location(0) vec4<f32> {
    if in.position.z > textureLoad(occlusion, vec2<i32>(in.position.xy), 0).r { discard; }
    return shade(in);
}
