struct Params {
  time: f32,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read_write> positions: array<vec4f>;

@compute @workgroup_size(3)
fn cs_main(@builtin(global_invocation_id) id: vec3u) {
  if id.x >= 3u {
    return;
  }

  let rest = array<vec4f, 3>(
    vec4f(0.0, 0.7, 0.0, 1.0),
    vec4f(-0.65, -0.55, 0.0, 1.0),
    vec4f(0.65, -0.55, 0.0, 1.0),
  );
  let scale = 0.7 + 0.3 * sin(params.time + f32(id.x));
  let angle = params.time * 0.35;
  let point = rest[id.x].xy * scale;
  positions[id.x] = vec4f(
    point.x * cos(angle) - point.y * sin(angle),
    point.x * sin(angle) + point.y * cos(angle),
    0.0,
    1.0,
  );
}
