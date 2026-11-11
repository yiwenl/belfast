@group(1) @binding(0) var<storage, read> segments: array<Segment>;
struct Vertex {
    @builtin(position) position: vec4<f32>,
    @location(0) grid_position: vec2<f32>,
    @location(1) base_position: vec3<f32>,
    @location(2) base_normal: vec3<f32>,
};
fn terrain_vertex(id: u32, upper: bool) -> Vertex {
    let side = u32(view.grid.x) - 1u;
    let square = id / 6u;
    let corners = array<vec2<u32>, 6>(vec2(0u, 0u), vec2(0u, 1u), vec2(1u, 0u), vec2(1u, 0u), vec2(0u, 1u), vec2(1u, 1u));
    let p = vec2<f32>(vec2(square % side, square / side) + corners[id % 6u]);
    var out: Vertex;
    let position = select(mesh_position(p), line_position(p), upper);
    out.position = view.matrix * vec4(position, 1.0);
    out.grid_position = p;
    return out;
}
fn base_vertex(id: u32) -> Vertex {
    let side = u32(view.grid.x) - 1u;
    let wall_vertices = side * 4u * 6u;
    let bottom = -view.style.y * 0.5 - view.base.x;
    let corners = array<vec2<u32>, 6>(vec2(0u, 0u), vec2(0u, 1u), vec2(1u, 0u), vec2(1u, 0u), vec2(0u, 1u), vec2(1u, 1u));
    var out: Vertex;
    if (id < wall_vertices) {
        let edge = id / (side * 6u);
        let corner = corners[id % 6u];
        let starts = array<vec2<f32>, 4>(vec2(0.0), vec2(f32(side), 0.0), vec2(f32(side)), vec2(0.0, f32(side)));
        let directions = array<vec2<f32>, 4>(vec2(1.0, 0.0), vec2(0.0, 1.0), vec2(-1.0, 0.0), vec2(0.0, -1.0));
        let direction = directions[edge];
        let p = starts[edge] + direction * f32((id / 6u) % side + corner.x);
        // Sample every boundary vertex so the walls stay joined to the eroding surface.
        out.base_position = mesh_position(p);
        if (corner.y == 0u) { out.base_position.y = bottom; }
        out.base_normal = vec3(direction.y, 0.0, -direction.x);
        out.grid_position = p;
    } else {
        let p = vec2<f32>(corners[id - wall_vertices]) * f32(side);
        out.base_position = mesh_position(p);
        out.base_position.y = bottom;
        out.base_normal = vec3(0.0, -1.0, 0.0);
        out.grid_position = p;
    }
    out.position = view.matrix * vec4(out.base_position, 1.0);
    return out;
}
@vertex
fn mesh_vertex(@builtin(vertex_index) id: u32) -> Vertex {
    let side = u32(view.grid.x) - 1u;
    let surface_vertices = side * side * 6u;
    if (id >= surface_vertices) { return base_vertex(id - surface_vertices); }
    return terrain_vertex(id, false);
}
@vertex
fn line_depth_vertex(@builtin(vertex_index) id: u32) -> Vertex {
    return terrain_vertex(id, true);
}
@group(0) @binding(2) var env_irradiance: texture_2d<f32>;
@group(0) @binding(3) var env_specular: texture_2d<f32>;
@group(0) @binding(4) var env_sampler: sampler;

fn srgb_to_linear(color: vec3<f32>) -> vec3<f32> {
    return select(pow((color + 0.055) / 1.055, vec3(2.4)), color / 12.92, color <= vec3(0.04045));
}
fn pbr_output(color: vec3<f32>) -> vec3<f32> {
    let x = max(color, vec3(0.0));
    // ACES fitted curve keeps HDR highlights within the SDR canvas range.
    let mapped = clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), vec3(0.0), vec3(1.0));
    if (OUTPUT_SRGB_ATTACHMENT) { return mapped; }
    return select(1.055 * pow(mapped, vec3(1.0 / 2.4)) - 0.055, mapped * 12.92, mapped <= vec3(0.0031308));
}
fn hdr_uv(world_dir: vec3<f32>) -> vec2<f32> {
    let d = normalize(vec3(
        dot(view.env_x.xyz, world_dir),
        dot(view.env_y.xyz, world_dir),
        dot(view.env_z.xyz, world_dir),
    ));
    return vec2(
        atan2(d.z, d.x) * 0.15915494 + 0.5,
        acos(clamp(d.y, -1.0, 1.0)) * 0.31830989,
    );
}
fn pbr(normal: vec3<f32>, position: vec3<f32>, shadow_position: vec3<f32>, light_vec: vec3<f32>, cone: f32) -> vec3<f32> {
    let light_dir = normalize(light_vec);
    let shadow = terrain_shadow(shadow_position);
    let env_gain = view.light_gain.w;
    // The HDR sun was split out for shadows; Environment must add it back independently of Spot.
    let sun_cosine = max(dot(normal, view.light.xyz), 0.0);
    let sun = sun_cosine * (env_gain + view.light_gain.x);
    let spot = max(dot(normal, light_dir), 0.0) * cone;
    let direct = (sun + spot) * shadow;
    let env = textureSampleLevel(env_irradiance, env_sampler, hdr_uv(normal), 0.0).rgb * env_gain;
    let view_dir = normalize(view.eye.xyz - position * view.eye.w);
    let ndotv = max(dot(normal, view_dir), 0.001);
    let reflection = reflect(-view_dir, normal);
    let albedo = srgb_to_linear(view.mesh_color.rgb);
    let fresnel_base = mix(vec3(0.04), albedo, view.surface.x);
    let roughness = clamp(view.light_gain.z, 0.0, 1.0);
    // The unfiltered grazing Fresnel made even fully rough dark surfaces turn glossy white.
    let fresnel_grazing = max(vec3(1.0 - roughness), fresnel_base);
    let fresnel = fresnel_base + (fresnel_grazing - fresnel_base) * pow(1.0 - ndotv, 5.0);
    // The stored specular is the sharpest prefilter. Roughness broadens it without erasing it.
    let gloss = 1.0 - roughness;
    let sharp = textureSampleLevel(env_specular, env_sampler, hdr_uv(reflection), 0.0).rgb * env_gain;
    let sun_highlight = smoothstep(0.9, 0.99, max(dot(reflection, view.light.xyz), 0.0));
    let rough_env = env + view.sun_color.rgb * env_gain * sun_cosine * shadow;
    let specular = mix(rough_env, sharp * mix(1.0, shadow, sun_highlight), gloss) * fresnel;
    let diffuse = (1.0 - view.surface.x) * (1.0 - fresnel) * (env + direct * view.sun_color.rgb);
    return albedo * diffuse + specular;
}
fn mesh_lit(shade: f32) -> vec3<f32> {
    // 0.3 is the fully lit surface, so the picked color is that face.
    return view.mesh_color.rgb * shade / 0.3;
}
@fragment
fn mesh_fragment(in: Vertex) -> @location(0) vec4<f32> {
    let terrain = world_position(in.grid_position);
    let contour = fract(terrain.y / view.material.z);
    let distance = min(contour, 1.0 - contour) * view.material.z;
    let line = 1.0 - smoothstep(0.0, max(fwidth(terrain.y) * 2.0, 0.000001), distance);
    let cover = clamp(line * view.material.w / 0.2, 0.0, 1.0);
    if (view.sun_color.w > 0.5) {
        if (dot(in.base_normal, in.base_normal) > 0.5) {
            let local = in.base_position + vec3(0.0, view.style.y * 0.5, 0.0);
            return vec4(pbr_output(pbr(in.base_normal, in.base_position, local, view.light.xyz, 0.0)), 1.0);
        }
        let position = mesh_position(in.grid_position);
        let light_to_fragment = normalize(position - view.light.xyz * view.light.w);
        let angle = acos(clamp(dot(-view.light.xyz, light_to_fragment), -1.0, 1.0));
        let cone = pow(max(0.0, 1.0 - angle / view.material.x), view.material.y) * view.light_gain.y;
        let lit = pbr(normal_at(in.grid_position), position, terrain, -light_to_fragment, cone);
        let shadow = terrain_shadow(terrain);
        return vec4(pbr_output(mix(lit, srgb_to_linear(view.contour_color.rgb) * shadow, cover)), 1.0);
    }
    if (dot(in.base_normal, in.base_normal) > 0.5) {
        // Keep the cut faces readable even outside the surface spotlight.
        let diffuse = max(0.0, dot(in.base_normal, view.light.xyz)) * view.light_gain.x;
        let local = in.base_position + vec3(0.0, view.style.y * 0.5, 0.0);
        let shade = 0.18 * mix(0.25, 1.0, diffuse) * terrain_shadow(local);
        return vec4(mesh_lit(shade), 1.0);
    }
    let position = mesh_position(in.grid_position);
    let light_to_fragment = normalize(position - view.light.xyz * view.light.w);
    let angle = acos(clamp(dot(-view.light.xyz, light_to_fragment), -1.0, 1.0));
    let spot = pow(max(0.0, 1.0 - angle / view.material.x), view.material.y);
    let diffuse = max(0.0, dot(normal_at(in.grid_position), -light_to_fragment)) * spot * view.light_gain.y * view.light_gain.x;
    let shadow = terrain_shadow(terrain);
    let shade = 0.3 * mix(0.4, 1.0, diffuse) * shadow;
    // Strength 0.2 is a solid stroke. Cast shadows dim it with the surface.
    return vec4(mix(mesh_lit(shade), view.contour_color.rgb * shadow, cover), 1.0);
}
@vertex
fn line_vertex(@builtin(vertex_index) vertex: u32, @builtin(instance_index) instance: u32) -> @builtin(position) vec4<f32> {
    let s = segments[instance];
    if (s.info.x < 0.5) { return vec4(2.0, 2.0, 2.0, 1.0); }
    let a = s.a.xy / s.a.w * view.paper.xy;
    let b = s.b.xy / s.b.w * view.paper.xy;
    let direction = (b - a) * view.screen.xy;
    let perpendicular = vec2(-direction.y, direction.x) / max(length(direction), 0.00001);
    let corners = array<vec2<f32>, 6>(vec2(0.0, -1.0), vec2(1.0, -1.0), vec2(0.0, 1.0), vec2(0.0, 1.0), vec2(1.0, -1.0), vec2(1.0, 1.0));
    let corner = corners[vertex];
    let clip = mix(s.a, s.b, corner.x);
    let offset = perpendicular * corner.y * view.style.z / view.screen.xy;
    return vec4(clip.xy * view.paper.xy + offset * clip.w, clip.z, clip.w);
}
@fragment
fn line_fragment() -> @location(0) vec4<f32> { return view.line_color; }

@vertex
fn debug_vertex(@builtin(vertex_index) id: u32) -> Vertex {
    let p = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    var out: Vertex;
    out.position = vec4(p[id], 0.0, 1.0);
    out.grid_position = p[id] * 0.5 + 0.5;
    return out;
}
@fragment
fn debug_fragment(in: Vertex) -> @location(0) vec4<f32> {
    let uv = vec2(in.grid_position.x, 1.0 - in.grid_position.y);
    let data = cell(vec2<i32>(uv * (view.grid.x - 1.0)));
    var value = data.x / view.grid.z;
    if (view.screen.w > 0.5 && view.screen.w < 1.5) { value = data.y / view.grid.z; }
    if (view.screen.w > 1.5 && view.screen.w < 2.5) { value = 0.5 + (data.x - data.y) / view.grid.z * 5.0 * view.grid.w; }
    if (view.screen.w > 2.5 && view.screen.w < 3.5) { value = data.z / view.grid.z * 3.0; }
    if (view.screen.w > 3.5) { value = data.w / view.grid.z * 3.0; }
    if (view.screen.w < 1.5 || view.screen.w > 2.5) { value *= view.grid.w; }
    if (any(uv < vec2(0.008)) || any(uv > vec2(0.992))) { value = 0.3; }
    return vec4(vec3(clamp(value, 0.0, 1.0)), 1.0);
}
