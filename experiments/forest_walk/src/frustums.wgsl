struct Camera { view_projection: mat4x4<f32>, viewport: vec4<f32> };
@group(0) @binding(0) var<uniform> camera: Camera;
struct Output {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};
// Twelve edges of the same clip-space cube used by CameraHelper / DrawCamera.
const corners = array<vec3<f32>, 8>(
    vec3(1, 1, 0), vec3(-1, 1, 0), vec3(-1, -1, 0), vec3(1, -1, 0),
    vec3(1, 1, 1), vec3(-1, 1, 1), vec3(-1, -1, 1), vec3(1, -1, 1)
);
const edges = array<u32, 24>(0,1, 1,2, 2,3, 3,0, 4,5, 5,6, 6,7, 7,4, 0,4, 1,5, 2,6, 3,7);
@vertex fn vs_main(
    @builtin(vertex_index) vertex: u32,
    @location(0) c0: vec4<f32>, @location(1) c1: vec4<f32>,
    @location(2) c2: vec4<f32>, @location(3) c3: vec4<f32>,
    @location(4) color: vec4<f32>,
) -> Output {
    let world = mat4x4(c0, c1, c2, c3) * vec4(corners[edges[vertex]], 1);
    var out: Output;
    out.position = camera.view_projection * vec4(world.xyz / world.w, 1);
    out.color = color;
    return out;
}
// Screen-space ribbons keep the current camera's lines 2 CSS pixels wide on
// high-DPI displays; the 151 recorded cameras still use the native line list.
@vertex fn vs_current(
    @builtin(vertex_index) vertex: u32,
    @location(0) c0: vec4<f32>, @location(1) c1: vec4<f32>,
    @location(2) c2: vec4<f32>, @location(3) c3: vec4<f32>,
    @location(4) color: vec4<f32>,
) -> Output {
    let quad = array<vec2<f32>, 6>(vec2(0,-0.5), vec2(1,-0.5), vec2(0,0.5), vec2(0,0.5), vec2(1,-0.5), vec2(1,0.5));
    let edge = vertex / 6u;
    let coord = quad[vertex % 6u];
    let inverse = mat4x4(c0, c1, c2, c3);
    let world_a = inverse * vec4(corners[edges[edge * 2u]], 1);
    let world_b = inverse * vec4(corners[edges[edge * 2u + 1u]], 1);
    let a = camera.view_projection * vec4(world_a.xyz / world_a.w, 1);
    let b = camera.view_projection * vec4(world_b.xyz / world_b.w, 1);
    let delta = (b.xy / b.w - a.xy / a.w) * camera.viewport.xy;
    let normal = vec2(-delta.y, delta.x) / max(length(delta), 0.0001);
    var out: Output;
    out.position = mix(a, b, coord.x);
    out.position.x += normal.x * coord.y * camera.viewport.z * 2. / camera.viewport.x * out.position.w;
    out.position.y += normal.y * coord.y * camera.viewport.z * 2. / camera.viewport.y * out.position.w;
    out.color = color;
    return out;
}
@fragment fn fs_main(in: Output) -> @location(0) vec4<f32> {
    var rgb = in.color.rgb;
    if camera.viewport.w > 0.5 {
        rgb = select(rgb / 12.92, pow((rgb + 0.055) / 1.055, vec3(2.4)), rgb > vec3(0.04045));
    }
    return vec4(rgb, 1);
}
