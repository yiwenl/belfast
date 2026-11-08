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
@fragment
fn mesh_fragment(in: Vertex) -> @location(0) vec4<f32> {
    let terrain = world_position(in.grid_position);
    let contour = fract(terrain.y / view.material.z);
    let distance = min(contour, 1.0 - contour) * view.material.z;
    let line = 1.0 - smoothstep(0.0, max(fwidth(terrain.y) * 2.0, 0.000001), distance);
    if (dot(in.base_normal, in.base_normal) > 0.5) {
        // Keep the cut faces readable even outside the surface spotlight.
        let diffuse = max(0.0, dot(in.base_normal, view.light.xyz));
        let local = in.base_position + vec3(0.0, view.style.y * 0.5, 0.0);
        let shade = 0.18 * mix(0.25, 1.0, diffuse) * terrain_shadow(local);
        return vec4(vec3(shade), 1.0);
    }
    let position = mesh_position(in.grid_position);
    let light_to_fragment = normalize(position - view.light.xyz * view.light.w);
    let angle = acos(clamp(dot(-view.light.xyz, light_to_fragment), -1.0, 1.0));
    let spot = pow(max(0.0, 1.0 - angle / view.material.x), view.material.y);
    let diffuse = max(0.0, dot(normal_at(in.grid_position), -light_to_fragment)) * spot;
    // Match erosion4's dark diffuse material and faint height contours in grayscale.
    let shade = (0.3 * mix(0.4, 1.0, diffuse) + line * view.material.w) * terrain_shadow(terrain);
    return vec4(vec3(shade), 1.0);
}
@vertex
fn line_vertex(@builtin(vertex_index) vertex: u32, @builtin(instance_index) instance: u32) -> @builtin(position) vec4<f32> {
    let s = segments[instance];
    if (s.info.x < 0.5) { return vec4(2.0, 2.0, 2.0, 1.0); }
    let a = s.a.xy / s.a.w;
    let b = s.b.xy / s.b.w;
    let direction = (b - a) * view.screen.xy;
    let perpendicular = vec2(-direction.y, direction.x) / max(length(direction), 0.00001);
    let corners = array<vec2<f32>, 6>(vec2(0.0, -1.0), vec2(1.0, -1.0), vec2(0.0, 1.0), vec2(0.0, 1.0), vec2(1.0, -1.0), vec2(1.0, 1.0));
    let corner = corners[vertex];
    let clip = mix(s.a, s.b, corner.x);
    let offset = perpendicular * corner.y * view.style.z / view.screen.xy;
    return vec4(clip.xy + offset * clip.w, clip.z, clip.w);
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
