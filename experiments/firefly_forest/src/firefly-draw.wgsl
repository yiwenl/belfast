struct Camera {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    sprite: vec4<f32>, // Size, sRGB attachment, particle opacity, oldest visible age.
    effects: vec4<f32>, // Shared brightness boost, particle glow, reserved.
};
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
    let intensity = boosted_flash(p.flash.y, camera.effects.x);
    if intensity <= 0. || camera.sprite.z <= 0. {
        // All six vertices lie outside the clip volume (z > w), so an unlit
        // billboard produces no fragments or additive blending work.
        return Vertex(vec4(2., 2., 2., 1.), vec2(0.), 0., vec3(0.));
    }
    let corners = array<vec2<f32>, 6>(vec2(-1,-1),vec2(1,-1),vec2(1,1),vec2(-1,-1),vec2(1,1),vec2(-1,1));
    let uv = corners[vertex];
    let variation = fract(f32(index) * 0.61803398875);
    let emission = firefly_emission(index, intensity);
    // Compute supplies a fixed-duration envelope independent of cycle speed.
    let brightness = emission.a * camera.sprite.z;
    let radius = mix(0.045, 0.07, variation) * camera.sprite.x;
    let center = camera.view * vec4(p.position_phase.xyz, 1.);
    var out: Vertex;
    out.position = camera.projection * (center + vec4(uv * radius, 0., 0.));
    out.uv = uv;
    out.brightness = brightness;
    out.color = emission.rgb;
    return out;
}
fn shade(in: Vertex) -> vec4<f32> {
    let r2 = dot(in.uv, in.uv);
    if r2 > 1. { discard; }
    let core = exp(-r2 * 100.);
    var halo = 0.;
    if camera.effects.y > 0. {
        halo = exp(-r2 * 6.) * 0.18 * (1. - smoothstep(0.6, 1., r2));
    }
    let core_color = mix(in.color, vec3(1., 1., 0.65), 0.28);
    let rgb = (core_color * core + in.color * halo) * in.brightness;
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
