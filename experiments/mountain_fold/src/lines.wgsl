@group(1) @binding(0) var<storage, read_write> segments: array<Segment>;
@group(1) @binding(1) var camera_depth: texture_depth_2d;

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

fn visible(p: vec2<f32>, rank: f32) -> bool {
    if (any(p < vec2(0.0)) || any(p > vec2(view.grid.x - 1.0))) { return false; }
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
    if (dot(normal, normalize(view.eye.xyz - world * view.eye.w)) < 0.25) { return false; }
    let light = max(0.0, dot(normal, view.light.xyz)) * terrain_shadow(world_position(p));
    let ink = smoothstep(0.0, 0.75, pow(clamp(light * view.style.x, 0.0, 1.0), 1.5));
    return rank < ink;
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
@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let count = view.counts.x * view.counts.y;
    if (id.x >= count) { return; }
    let ring = id.x / view.counts.y;
    let step = id.x % view.counts.y;
    let a = stroke_point(ring, step * 2u);
    let b = stroke_point(ring, step * 2u + 1u);
    let c = stroke_point(ring, step * 2u + 2u);
    let rank = (f32(ring % 8u) + 0.5) / 8.0;
    write_segment(id.x * 2u, a, b, rank, ring);
    write_segment(id.x * 2u + 1u, b, c, rank, ring);
}
