struct VertexInput {
  @location(0) position: vec2<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> @builtin(position) vec4<f32> {
  return vec4<f32>(input.position, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
  return vec4<f32>(0.9, 0.35, 0.2, 1.0);
}
