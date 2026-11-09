struct View {
    matrix: mat4x4<f32>,
    eye: vec4<f32>, // eye position (w=1) or direction toward the orthographic camera (w=0)
    light: vec4<f32>,
    style: vec4<f32>, // line exposure, layer spacing, line width, shadows
    material: vec4<f32>,
    grid: vec4<f32>,
    screen: vec4<f32>,
    counts: vec4<u32>, // curves, hatch columns or Voronoi segments; samples; seed; mode
    line_color: vec4<f32>,
    base: vec4<f32>, // thickness in world units, inverted ink density (1 = lit faces get more lines)
    stroke: vec4<f32>, // wobble amplitude/wavelength, hatch spacing, min line elevation
    inverse_matrix: mat4x4<f32>,
    paper: vec4<f32>, // printable width/height as fractions of the page
    contour_color: vec4<f32>,
    mesh_color: vec4<f32>,
};
struct Segment { a: vec4<f32>, b: vec4<f32>, info: vec4<f32> };
@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<storage, read> heights: array<vec4<f32>>;

fn cell(p: vec2<i32>) -> vec4<f32> {
    let q = vec2<u32>(clamp(p, vec2(0), vec2(i32(view.grid.x) - 1)));
    return heights[q.x + q.y * u32(view.grid.x)];
}
fn selected_height(p: vec2<i32>) -> f32 {
    let h = cell(p);
    return mix(h.x, h.y, view.screen.z);
}
fn height_at(p: vec2<f32>) -> f32 {
    let q = vec2<i32>(floor(p));
    let f = fract(p);
    let a = selected_height(q);
    let b = selected_height(q + vec2(1, 0));
    let c = selected_height(q + vec2(0, 1));
    let d = selected_height(q + vec2(1, 1));
    // Match the mesh diagonal so ink cannot sink below its own depth surface.
    if (f.x + f.y <= 1.0) { return a * (1.0 - f.x - f.y) + b * f.x + c * f.y; }
    return d * (f.x + f.y - 1.0) + b * (1.0 - f.y) + c * (1.0 - f.x);
}
fn world_position(p: vec2<f32>) -> vec3<f32> {
    let xz = (p - vec2((view.grid.x - 1.0) * 0.5)) * view.grid.y;
    return vec3(xz.x, height_at(p) * view.grid.y, xz.y);
}
fn mesh_position(p: vec2<f32>) -> vec3<f32> {
    return world_position(p) - vec3(0.0, view.style.y * 0.5, 0.0);
}
fn line_position(p: vec2<f32>) -> vec3<f32> {
    return world_position(p) + vec3(0.0, view.style.y * 0.5, 0.0);
}
fn normal_at(p: vec2<f32>) -> vec3<f32> {
    return normalize(vec3(height_at(p - vec2(1.0, 0.0)) - height_at(p + vec2(1.0, 0.0)), 2.0,
        height_at(p - vec2(0.0, 1.0)) - height_at(p + vec2(0.0, 1.0))));
}
fn terrain_shadow(position: vec3<f32>) -> f32 {
    if (view.style.w < 0.5) { return 1.0; }
    for (var i = 1u; i < 45u; i++) {
        let ray = position + view.light.xyz * (f32(i) * 0.19);
        let p = ray.xz / view.grid.y + vec2((view.grid.x - 1.0) * 0.5);
        if (any(p < vec2(0.0)) || any(p > vec2(view.grid.x - 1.0))) { break; }
        if (height_at(p) * view.grid.y > ray.y + 0.035) { return 0.4; }
    }
    return 1.0;
}
