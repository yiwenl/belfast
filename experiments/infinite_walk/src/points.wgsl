@group(1) @binding(0) var shadow_map: texture_depth_2d;
@group(1) @binding(1) var shadow_sampler: sampler_comparison;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) point_coord: vec2<f32>,
    @location(1) @interpolate(flat) color: vec3<f32>,
}

fn point_corner(vertex: u32) -> vec2<f32> {
    let corners = array<vec2<f32>, 6>(
        vec2(-0.5, -0.5), vec2(0.5, -0.5), vec2(-0.5, 0.5),
        vec2(-0.5, 0.5), vec2(0.5, -0.5), vec2(0.5, 0.5),
    );
    return corners[vertex];
}

fn shadow_visibility(center: vec3<f32>) -> f32 {
    let clip = camera.light_view_projection * vec4(center, 1.0);
    if clip.w <= 0.0 || clip.z < 0.0 || clip.z > clip.w || any(abs(clip.xy) > vec2(clip.w)) {
        return 1.0;
    }
    let projected = clip.xyz / clip.w;
    let uv = vec2(projected.x * 0.5 + 0.5, 0.5 - projected.y * 0.5);
    let depth = projected.z - camera.shadow.y;
    let texel = 1.0 / camera.shadow.x;
    var visibility = 0.0;
    // A fixed, wider tent filter reduces brightness jumps as particle shadows cross texels.
    for (var y = -2; y <= 2; y++) {
        for (var x = -2; x <= 2; x++) {
            let weight = f32((3 - abs(x)) * (3 - abs(y)));
            visibility += weight * textureSampleCompareLevel(
                shadow_map,
                shadow_sampler,
                uv + vec2(f32(x), f32(y)) * texel,
                depth,
            );
        }
    }
    return mix(1.0, visibility / 81.0, camera.shadow.z);
}

fn billboard_vertex(vertex: u32, center: vec3<f32>, color: vec4<f32>, size: f32) -> VertexOut {
    var out: VertexOut;
    let coord = point_corner(vertex);
    var clip = camera.view_projection * vec4(center, 1.0);
    // Keep points behind the near plane from expanding across the viewport.
    if clip.w <= 0.0 || clip.z < 0.0 {
        clip = vec4(2.0, 2.0, 2.0, 1.0);
    } else {
        // Expand along camera-local X/Y before perspective division, so nearby points grow.
        clip.x += coord.x * size * camera.billboard.x;
        clip.y += coord.y * size * camera.billboard.y;
    }
    out.position = clip;
    out.point_coord = coord;
    var illumination = camera.light_shape.w;
    if inside_light_cylinder(center) {
        let offset = center - camera.light.xyz;
        let edge = 1.0 - smoothstep(camera.light_shape.z, camera.light_shape.y, length(offset.xz));
        let depth_squared = offset.y * offset.y;
        let falloff = 1.0 - depth_squared / (camera.light_shape.x * camera.light_shape.x);
        var direct = camera.light.w * edge * falloff * falloff / (1.0 + depth_squared);
        if camera.shadow.z > 0.0 {
            // shortcut: receive at particle centers; use surface geometry for per-pixel shadow detail.
            direct *= shadow_visibility(center);
        }
        illumination += direct;
    }
    let albedo = select(color.rgb, vec3(1.0), camera.appearance.x > 0.5);
    out.color = srgb_to_linear(albedo) * illumination;
    return out;
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex: u32,
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
) -> VertexOut {
    return billboard_vertex(vertex, wrapped_position(position), color, camera.billboard.z);
}

@vertex
fn vs_particles(
    @builtin(vertex_index) vertex: u32,
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) size: f32,
) -> VertexOut {
    return billboard_vertex(vertex, position, color, size);
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

struct ShadowVertex {
    @builtin(position) position: vec4<f32>,
    @location(0) point_coord: vec2<f32>,
}

fn shadow_vertex(vertex: u32, center: vec3<f32>, size: f32) -> ShadowVertex {
    let coord = point_corner(vertex);
    // Light-facing discs give particles a solid circular silhouette from the downward light.
    let world = center + vec3(coord.x, 0.0, coord.y) * size;
    var out: ShadowVertex;
    out.position = camera.light_view_projection * vec4(world, 1.0);
    out.point_coord = coord;
    return out;
}

@vertex
fn vs_shadow(@builtin(vertex_index) vertex: u32, @location(0) position: vec3<f32>) -> ShadowVertex {
    return shadow_vertex(vertex, wrapped_position(position), camera.billboard.z);
}

@vertex
fn vs_particle_shadow(
    @builtin(vertex_index) vertex: u32,
    @location(0) position: vec3<f32>,
    @location(2) size: f32,
) -> ShadowVertex {
    return shadow_vertex(vertex, position, size);
}

@fragment
fn fs_shadow(in: ShadowVertex) {
    let disk = in.position.xy * (2.0 / camera.shadow.x) - 1.0;
    if dot(disk, disk) > 1.0 || length(in.point_coord) > 0.5 { discard; }
}

struct ShadowPreviewVertex {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_shadow_preview(@builtin(vertex_index) vertex: u32) -> ShadowPreviewVertex {
    let positions = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    let position = positions[vertex];
    var out: ShadowPreviewVertex;
    out.position = vec4(position, 0.0, 1.0);
    out.uv = vec2(position.x * 0.5 + 0.5, 0.5 - position.y * 0.5);
    return out;
}

@fragment
fn fs_shadow_preview(in: ShadowPreviewVertex) -> @location(0) vec4<f32> {
    let size = textureDimensions(shadow_map);
    let pixel = clamp(vec2<i32>(in.uv * vec2<f32>(size)), vec2(0), vec2<i32>(size) - vec2(1));
    let depth = textureLoad(shadow_map, pixel, 0);
    var color = vec3(depth);
    if camera.billboard.w > 0.5 { color = srgb_to_linear(color); }
    return vec4(color, 1.0);
}
