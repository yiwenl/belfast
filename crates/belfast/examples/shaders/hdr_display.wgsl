struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@location(0) position: vec2<f32>) -> VertexOutput {
  var output: VertexOutput;
  output.position = vec4<f32>(position, 0.0, 1.0);
  output.uv = position * 0.5 + vec2<f32>(0.5);
  return output;
}

// Linear luminance ramp: 0 at the left, 8 at the right. 1.0 is SDR white
// (x = 1/8) and is marked with a red tick. Write linear; the surface color
// space (scRGB / extended sRGB) treats 1.0 as reference white.
const MAX_LUMA: f32 = 8.0;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
  let luma = input.uv.x * MAX_LUMA;
  let tick = abs(input.uv.x - 1.0 / MAX_LUMA) < 0.004;
  let color = select(vec3<f32>(luma), vec3<f32>(1.0, 0.15, 0.1), tick);
  return vec4<f32>(color, 1.0);
}
