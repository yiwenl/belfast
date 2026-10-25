struct Camera {
    view_projection: mat4x4<f32>,
    // Physical viewport width/height, physical point diameter, sRGB attachment flag.
    viewport: vec4<f32>,
}
@group(0) @binding(0) var<uniform> camera: Camera;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) point_coord: vec2<f32>,
    @location(1) @interpolate(flat) color: vec3<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex: u32,
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
) -> VertexOut {
    let corners = array<vec2<f32>, 6>(
        vec2(-0.5, -0.5), vec2(0.5, -0.5), vec2(-0.5, 0.5),
        vec2(-0.5, 0.5), vec2(0.5, -0.5), vec2(0.5, 0.5),
    );
    var out: VertexOut;
    let coord = corners[vertex];
    var clip = camera.view_projection * vec4(position, 1.0);
    // Keep points behind the near plane from expanding across the viewport.
    if clip.w <= 0.0 || clip.z < 0.0 {
        clip = vec4(2.0, 2.0, 2.0, 1.0);
    } else {
        clip.x += coord.x * camera.viewport.z * 2.0 / camera.viewport.x * clip.w;
        clip.y += coord.y * camera.viewport.z * 2.0 / camera.viewport.y * clip.w;
    }
    out.position = clip;
    out.point_coord = coord;
    out.color = color.rgb;
    return out;
}

fn srgb_to_linear(color: vec3<f32>) -> vec3<f32> {
    return select(
        pow((color + vec3(0.055)) / 1.055, vec3(2.4)),
        color / 12.92,
        color <= vec3(0.04045),
    );
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    if length(in.point_coord) > 0.5 { discard; }
    var color = in.color;
    if camera.viewport.w > 0.5 { color = srgb_to_linear(color); }
    return vec4(color, 1.0);
}
