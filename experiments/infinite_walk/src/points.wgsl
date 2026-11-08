@group(1) @binding(0) var shadow_map: texture_depth_2d;
@group(1) @binding(1) var shadow_sampler: sampler_comparison;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) point_coord: vec2<f32>,
    @location(1) @interpolate(flat) color: vec3<f32>,
}

fn point_corner(vertex: u32) -> vec2<f32> {
    let corners = array<vec2<f32>, 4>(
        vec2(-0.5, -0.5), vec2(0.5, -0.5), vec2(-0.5, 0.5),
        vec2(0.5, 0.5),
    );
    return corners[vertex];
}

fn shadow_coordinates(beam: vec3<f32>) -> vec3<f32> {
    return vec3(beam.x * 0.5 + 0.5, 0.5 - beam.y * 0.5, beam.z - camera.shadow.y);
}

fn beam_edge(radial: vec2<f32>) -> f32 {
    return 1.0 - smoothstep(camera.light_shape.z, camera.light_shape.y, length(radial) * camera.light_shape.y);
}

fn light_at_depth(depth: f32) -> f32 {
    let distance = depth * camera.light_shape.x;
    let falloff = 1.0 - depth * depth;
    return camera.light.w * falloff * falloff / (1.0 + distance * distance);
}

fn shadow_visibility(beam: vec3<f32>) -> f32 {
    let coords = shadow_coordinates(beam);
    let texel = 1.0 / camera.shadow.x;
    var visibility = 0.0;
    // Reconstruct the same 5x5 [1,2,3,2,1] tent using nine bilinear samples.
    // Account for subtexel phase before pairing adjacent texel weights; merely
    // averaging fixed sample offsets would change the kernel while moving.
    let pixel = coords.xy * camera.shadow.x - vec2(0.5);
    let base = floor(pixel) + vec2(0.5);
    let f = fract(pixel);
    let wx = vec3(3.0 - 2.0 * f.x, 5.0, 1.0 + 2.0 * f.x);
    let wy = vec3(3.0 - 2.0 * f.y, 5.0, 1.0 + 2.0 * f.y);
    let ox = vec3(-2.0 + (2.0 - f.x) / wx.x, (2.0 + f.x) / 5.0, 2.0 + f.x / wx.z);
    let oy = vec3(-2.0 + (2.0 - f.y) / wy.x, (2.0 + f.y) / 5.0, 2.0 + f.y / wy.z);
    for (var y = 0u; y < 3u; y++) {
        for (var x = 0u; x < 3u; x++) {
            visibility += wx[x] * wy[y] * textureSampleCompareLevel(
                shadow_map,
                shadow_sampler,
                (base + vec2(ox[x], oy[y])) * texel,
                coords.z,
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
    let beam = light_space(center);
    if inside_light_cylinder(beam) {
        illumination += beam_edge(beam.xy) * light_at_depth(beam.z);
        if camera.shadow.z > 0.0 {
            // shortcut: receive at particle centers; use surface geometry for per-pixel shadow detail.
            illumination *= shadow_visibility(beam);
        }
    }
    let albedo = select(color.rgb, vec3(1.0), camera.appearance.x > 0.5);
    out.color = srgb_to_linear(albedo) * illumination;
    return out;
}

@vertex
fn vs_backdrop(
    @builtin(vertex_index) vertex: u32,
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
) -> VertexOut {
    // Three depth layers share the walk's rebase, including its lateral drift.
    let tile = f32(vertex / 4u);
    let center = wrapped_position(position) + tile * vec3(camera.walk.w, 0.0, -camera.walk.z)
        + vec3(0.0, camera.backdrop_y_offset, 0.0);
    let coord = point_corner(vertex % 4u);
    var clip = camera.view_projection * vec4(center, 1.0);
    var out: VertexOut;
    let distance = max(0.0, clip.w);
    if clip.w <= 0.0 || clip.z < 0.0 {
        clip = vec4(2.0, 2.0, 2.0, 1.0);
    } else {
        clip.x += coord.x * camera.backdrop_point_size * camera.billboard.x;
        clip.y += coord.y * camera.backdrop_point_size * camera.billboard.y;
    }
    let haze = 1.0 - exp(-distance * 0.065);
    let albedo = select(color.rgb, vec3(0.6), camera.appearance.x > 0.5);
    let near_color = srgb_to_linear(albedo) * 0.32;
    let mist = srgb_to_linear(vec3(9.0, 19.0, 21.0) / 255.0);
    out.position = clip;
    out.point_coord = coord;
    out.color = mix(near_color, mist, haze);
    return out;
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex: u32,
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) copy_direction: vec3<f32>,
) -> VertexOut {
    let center = forest_position(position, copy_direction, vertex >= 4u);
    return billboard_vertex(vertex % 4u, center, color, camera.billboard.z);
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
    // Expand in the light's image plane so shadow discs always face the source.
    let beam = light_space(center);
    var out: ShadowVertex;
    out.position = vec4(beam.xy + coord * (size / camera.light_shape.y), beam.z, 1.0);
    out.point_coord = coord;
    return out;
}

@vertex
fn vs_shadow(
    @builtin(vertex_index) vertex: u32,
    @location(0) position: vec3<f32>,
    @location(2) copy_direction: vec3<f32>,
) -> ShadowVertex {
    let center = forest_position(position, copy_direction, vertex >= 4u);
    return shadow_vertex(vertex % 4u, center, camera.billboard.z);
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

struct ScreenVertex {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_fullscreen(@builtin(vertex_index) vertex: u32) -> ScreenVertex {
    let positions = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    let position = positions[vertex];
    var out: ScreenVertex;
    out.position = vec4(position, 0.0, 1.0);
    out.uv = vec2(position.x * 0.5 + 0.5, 0.5 - position.y * 0.5);
    return out;
}

@fragment
fn fs_shadow_preview(in: ScreenVertex) -> @location(0) vec4<f32> {
    let size = textureDimensions(shadow_map);
    let pixel = clamp(vec2<i32>(in.uv * vec2<f32>(size)), vec2(0), vec2<i32>(size) - vec2(1));
    let depth = textureLoad(shadow_map, pixel, 0);
    var color = vec3(depth);
    if camera.billboard.w > 0.5 { color = srgb_to_linear(color); }
    return vec4(color, 1.0);
}
