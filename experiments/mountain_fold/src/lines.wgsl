@group(1) @binding(0) var<storage, read_write> segments: array<Segment>;
@group(1) @binding(1) var camera_depth: texture_depth_2d;
struct VoronoiSegment { points: vec4<f32>, info: vec4<f32>, direction: vec4<f32> }; // info: rank, stroke ID, endpoint clearances; direction.xy: cell direction
@group(1) @binding(2) var<storage, read> voronoi_segments: array<VoronoiSegment>;

fn stroke_random(knot: u32, ring: u32) -> f32 {
    var hash = knot * 374761393u + ring * 668265263u + view.counts.z;
    hash = (hash ^ (hash >> 13u)) * 1274126177u;
    hash ^= hash >> 16u;
    return f32(hash & 0x00ffffffu) / 16777215.0 * 2.0 - 1.0;
}
fn stroke_point(ring: u32, sample: u32) -> vec2<f32> {
    let phase = f32(sample % (view.counts.y * 2u)) / f32(view.counts.y * 2u);
    let angle = phase * 6.2831853;
    let radius = (f32(ring) + 0.5) / f32(view.counts.x) * 0.707107 * (view.grid.x - 1.0);
    var offset = 0.0;
    if (view.stroke.x > 0.0) {
        let knots = max(4u, u32(round(radius * view.grid.y * 6.2831853 / view.stroke.y)));
        let t = phase * f32(knots);
        let knot = u32(floor(t));
        let f = fract(t);
        // Wrap the noise as well as the angle so adjacent segments and the circle seam join.
        let noise = mix(stroke_random(knot % knots, ring), stroke_random((knot + 1u) % knots, ring), f * f * (3.0 - 2.0 * f));
        offset = noise * min(view.stroke.x / view.grid.y, radius * 0.4);
    }
    return vec2((view.grid.x - 1.0) * 0.5) + vec2(cos(angle), sin(angle)) * (radius + offset);
}

fn ink(light: f32) -> f32 {
    let lit = smoothstep(0.05, 0.8, clamp(light * view.style.x, 0.0, 1.0));
    return select(1.0 - lit, lit, view.base.y > 0.5);
}
fn surface_light(p: vec2<f32>, normal: vec3<f32>) -> f32 {
    if (view.sun_color.w > 0.5) {
        // Match the displayed PBR brightness before decorative mesh contours.
        let color = linear_to_srgb(tone_map(surface_pbr(p, normal)));
        return dot(color, vec3(0.2126, 0.7152, 0.0722));
    }
    return max(0.0, dot(normal, view.light.xyz)) * terrain_shadow(world_position(p));
}
fn voronoi_point(p: vec2<f32>, direction: vec2<f32>, clearance: f32, stroke: u32) -> vec2<f32> {
    if (view.stroke.x <= 0.0) { return p; }
    let t = dot(p, direction) * view.grid.y / view.stroke.y;
    let knot = i32(floor(t));
    let f = fract(t);
    let noise = mix(stroke_random(bitcast<u32>(knot), stroke), stroke_random(bitcast<u32>(knot + 1), stroke), f * f * (3.0 - 2.0 * f));
    // Limit displacement to the cell's interior, including the endpoints at its border.
    let amplitude = min(view.stroke.x / view.grid.y, clearance);
    return p + vec2(-direction.y, direction.x) * noise * amplitude;
}
fn above_floor(p: vec2<f32>) -> bool {
    return height_at(p) * view.grid.y >= view.stroke.w;
}
fn visible(p: vec2<f32>, rank: f32) -> bool {
    if (any(p < vec2(0.0)) || any(p > vec2(view.grid.x - 1.0)) || !above_floor(p)) { return false; }
    let world = line_position(p);
    let clip = view.matrix * vec4(world, 1.0);
    if (clip.w <= 0.0) { return false; }
    let ndc = clip.xyz / clip.w;
    if (any(abs(ndc.xy) > vec2(1.0)) || ndc.z < 0.0 || ndc.z > 1.0) { return false; }
    let uv = vec2(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    // Cover the subpixel footprint so steep visible slopes do not self-occlude in orthographic views.
    let screen = vec2<i32>(floor(uv * view.screen.xy - vec2(0.5)));
    let edge = vec2<i32>(view.screen.xy) - vec2(1);
    let lo = clamp(screen, vec2(0), edge);
    let hi = clamp(screen + vec2(1), vec2(0), edge);
    let depth = max(max(textureLoad(camera_depth, lo, 0), textureLoad(camera_depth, hi, 0)),
        max(textureLoad(camera_depth, vec2(lo.x, hi.y), 0), textureLoad(camera_depth, vec2(hi.x, lo.y), 0)));
    if (ndc.z > depth + 0.000035) { return false; }
    let normal = normal_at(p);
    let facing = select(0.25, 0.0, view.counts.w == 2u);
    if (dot(normal, normalize(view.eye.xyz - world * view.eye.w)) < facing) { return false; }
    let light = surface_light(p, normal);
    return rank < ink(light);
}
fn write_segment(index: u32, start: vec2<f32>, end: vec2<f32>, rank: f32, ring: u32) {
    var a = start;
    var b = end;
    let va = visible(a, rank);
    let vb = visible(b, rank);
    var result: Segment;
    result.info = vec4(0.0, f32(ring), 0.0, 0.0);
    if (va || vb) {
        if (va != vb) {
            var lo = a;
            var hi = b;
            for (var i = 0; i < 6; i++) {
                let mid = (lo + hi) * 0.5;
                if (visible(mid, rank) == va) { lo = mid; } else { hi = mid; }
            }
            if (va) { b = lo; } else { a = hi; }
        }
        result.a = view.matrix * vec4(line_position(a), 1.0);
        result.b = view.matrix * vec4(line_position(b), 1.0);
        result.info.x = 1.0;
    }
    segments[index] = result;
}

fn hatch_sample(ndc: vec2<f32>, rank: f32) -> vec2<f32> {
    let uv = ndc * vec2(0.5, -0.5) + vec2(0.5);
    let pixel = clamp(vec2<i32>(floor(uv * view.screen.xy)), vec2(0), vec2<i32>(view.screen.xy) - vec2(1));
    let depth = textureLoad(camera_depth, pixel, 0);
    if (depth >= 1.0) { return vec2(depth, 0.0); }
    // Reconstruct at the depth texel center to identify the visible upper terrain.
    let sample_ndc = (vec2<f32>(pixel) + vec2(0.5)) / view.screen.xy * vec2(2.0, -2.0) + vec2(-1.0, 1.0);
    let projected = view.inverse_matrix * vec4(sample_ndc, depth, 1.0);
    let world = projected.xyz / projected.w;
    let p = world.xz / view.grid.y + vec2((view.grid.x - 1.0) * 0.5);
    if (any(p < vec2(0.0)) || any(p > vec2(view.grid.x - 1.0)) || !above_floor(p)) { return vec2(depth, 0.0); }
    if (abs(world.y - line_position(p).y) > view.grid.y * 0.5) { return vec2(depth, 0.0); }
    let normal = normal_at(p);
    if (dot(normal, normalize(view.eye.xyz - world * view.eye.w)) <= 0.0) { return vec2(depth, 0.0); }
    let light = surface_light(p, normal);
    return vec2(depth, select(0.0, 1.0, rank < ink(light)));
}
fn write_hatch(column: u32, step: u32) {
    let x = (f32(column) - f32(view.counts.x / 2u)) * view.stroke.z;
    var a = vec2(x, 1.0 - f32(step) / f32(view.counts.y) * 2.0);
    var b = vec2(x, 1.0 - f32(step + 1u) / f32(view.counts.y) * 2.0);
    let order = array<u32, 8>(0u, 4u, 2u, 6u, 1u, 5u, 3u, 7u);
    // Anchor the density pattern at the center so resizing only adds/removes edge columns.
    let band = (column + 8u - (view.counts.x / 2u) % 8u) % 8u;
    let rank = (f32(order[band]) + 0.5) / 8.0;
    var sa = hatch_sample(a, rank);
    var sb = hatch_sample(b, rank);
    var result: Segment;
    result.info = vec4(0.0, f32(column), 0.0, 0.0);
    if (sa.y > 0.5 || sb.y > 0.5) {
        if (sa.y != sb.y) {
            var lo = a;
            var hi = b;
            for (var i = 0u; i < 6u; i++) {
                let mid = (lo + hi) * 0.5;
                if (hatch_sample(mid, rank).y == sa.y) { lo = mid; } else { hi = mid; }
            }
            if (sa.y > 0.5) { b = lo; sb = hatch_sample(b, rank); }
            else { a = hi; sa = hatch_sample(a, rank); }
        }
        result.a = vec4(a, sa.x, 1.0);
        result.b = vec4(b, sb.x, 1.0);
        result.info.x = 1.0;
    }
    // Keep each column contiguous for the shared SVG path joiner.
    segments[column * view.counts.y + step] = result;
}
@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (view.counts.w == 2u) {
        if (id.x >= view.counts.x) { return; }
        let line = voronoi_segments[id.x];
        // Curved strokes share their cell's direction so wobble stays continuous across joints.
        let direction = line.direction.xy;
        let stroke = u32(line.info.y);
        let a = voronoi_point(line.points.xy, direction, line.info.z, stroke);
        let b = voronoi_point(line.points.zw, direction, line.info.w, stroke);
        write_segment(id.x, a, b, line.info.x, stroke);
        return;
    }
    let count = view.counts.x * view.counts.y;
    if (id.x >= count) { return; }
    let ring = id.x / view.counts.y;
    let step = id.x % view.counts.y;
    if (view.counts.w == 1u) {
        write_hatch(ring, step);
        return;
    }
    let a = stroke_point(ring, step * 2u);
    let b = stroke_point(ring, step * 2u + 1u);
    let c = stroke_point(ring, step * 2u + 2u);
    let rank = (f32(ring % 8u) + 0.5) / 8.0;
    write_segment(id.x * 2u, a, b, rank, ring);
    write_segment(id.x * 2u + 1u, b, c, rank, ring);
}
