struct LightUniforms {
  viewProj: mat4x4<f32>,
  cameraRight: vec4<f32>,
  cameraUp: vec4<f32>,
}

@group(0) @binding(0) var<uniform> light: LightUniforms;

struct VertexInput {
  @location(0) localCorner: vec4<f32>,
  @location(1) instancePosSize: vec4<f32>,
}

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) discUv: vec2<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
  var output: VertexOutput;
  let size = input.instancePosSize.w;
  let worldPos =
    input.instancePosSize.xyz +
    light.cameraRight.xyz * input.localCorner.x * size +
    light.cameraUp.xyz * input.localCorner.y * size;
  output.position = light.viewProj * vec4<f32>(worldPos, 1.0);
  output.discUv = input.localCorner.zw;
  return output;
}

// Depth-only pass: discard outside the disc so the occluder silhouette
// matches the circular billboards drawn in the main pass.
@fragment
fn fs_main(input: VertexOutput) {
  if (length(input.discUv - vec2(0.5)) > 0.5) {
    discard;
  }
}
