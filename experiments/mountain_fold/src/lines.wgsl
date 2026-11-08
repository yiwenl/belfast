@group(1) @binding(0) var<storage, read_write> segments: array<Segment>;
@group(1) @binding(1) var camera_depth: texture_depth_2d;

fn visible(p: vec2<f32>, rank: f32) -> bool {
    if (any(p < vec2(0.0)) || any(p > vec2(view.grid.x - 1.0))) { return false; }
    let world = world_position(p);
    let clip = view.matrix * vec4(world, 1.0);
    if (clip.w <= 0.0) { return false; }
    let ndc = clip.xyz / clip.w;
    if (any(abs(ndc.xy) > vec2(1.0)) || ndc.z < 0.0 || ndc.z > 1.0) { return false; }
    let uv = vec2(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    let screen = vec2<i32>(uv * view.screen.xy) + vec2(i32(view.screen.x), 0);
    let depth = textureLoad(camera_depth, screen, 0);
    if (ndc.z > depth + 0.000035) { return false; }
    let normal = normal_at(p);
    if (dot(normal, normalize(view.eye.xyz - world)) < 0.25) { return false; }
    let light = max(0.0, dot(normal, view.light.xyz)) * terrain_shadow(world);
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
        result.a = view.matrix * vec4(world_position(a), 1.0);
        result.b = view.matrix * vec4(world_position(b), 1.0);
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
    let radius = (f32(ring) + 0.5) / f32(view.counts.x) * 0.707107 * (view.grid.x - 1.0);
    let center = vec2((view.grid.x - 1.0) * 0.5);
    let angle = f32(step) / f32(view.counts.y) * 6.2831853;
    let next = f32(step + 1u) / f32(view.counts.y) * 6.2831853;
    let mid = (angle + next) * 0.5;
    let a = center + vec2(cos(angle), sin(angle)) * radius;
    let b = center + vec2(cos(mid), sin(mid)) * radius;
    let c = center + vec2(cos(next), sin(next)) * radius;
    let rank = (f32(ring % 8u) + 0.5) / 8.0;
    write_segment(id.x * 2u, a, b, rank, ring);
    write_segment(id.x * 2u + 1u, b, c, rank, ring);
}
