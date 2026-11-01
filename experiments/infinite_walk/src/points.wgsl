struct Camera {
    view_projection: mat4x4<f32>,
    // Projection X/Y scale, world-space point diameter, sRGB attachment flag.
    billboard: vec4<f32>,
    // Camera Z, distance behind camera, loop length, X drift per loop.
    walk: vec4<f32>,
    // White-particle flag, followed by uniform alignment padding.
    appearance: vec4<f32>,
    // World position and intensity; the spotlight always points down world -Y.
    light: vec4<f32>,
    // Range, inner/outer cone cosines, ambient illumination.
    light_shape: vec4<f32>,
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
    let loops = floor((camera.walk.x + camera.walk.y - position.z) / camera.walk.z);
    // Repeat the preprocessed cloud along birdsong's slanted walking direction.
    let center = position + vec3(-loops * camera.walk.w, 0.0, loops * camera.walk.z);
    var clip = camera.view_projection * vec4(center, 1.0);
    // Keep points behind the near plane from expanding across the viewport.
    if clip.w <= 0.0 || clip.z < 0.0 {
        clip = vec4(2.0, 2.0, 2.0, 1.0);
    } else {
        // Expand along camera-local X/Y before perspective division, so nearby points grow.
        clip.x += coord.x * camera.billboard.z * camera.billboard.x;
        clip.y += coord.y * camera.billboard.z * camera.billboard.y;
    }
    out.position = clip;
    out.point_coord = coord;
    let from_light = center - camera.light.xyz;
    let distance_squared = dot(from_light, from_light);
    let cone_angle = -from_light.y / max(sqrt(distance_squared), 0.0001);
    let cone = smoothstep(camera.light_shape.z, camera.light_shape.y, cone_angle);
    let falloff = max(1.0 - distance_squared / (camera.light_shape.x * camera.light_shape.x), 0.0);
    let illumination = camera.light_shape.w
        + camera.light.w * cone * falloff * falloff / (1.0 + distance_squared);
    let albedo = select(color.rgb, vec3(1.0), camera.appearance.x > 0.5);
    out.color = srgb_to_linear(albedo) * illumination;
    return out;
}

fn srgb_to_linear(color: vec3<f32>) -> vec3<f32> {
    return select(
        pow((color + vec3(0.055)) / 1.055, vec3(2.4)),
        color / 12.92,
        color <= vec3(0.04045),
    );
}

fn linear_to_srgb(color: vec3<f32>) -> vec3<f32> {
    return select(
        1.055 * pow(color, vec3(1.0 / 2.4)) - vec3(0.055),
        color * 12.92,
        color <= vec3(0.0031308),
    );
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    if length(in.point_coord) > 0.5 { discard; }
    var color = in.color;
    if camera.billboard.w < 0.5 { color = linear_to_srgb(color); }
    return vec4(color, 1.0);
}
