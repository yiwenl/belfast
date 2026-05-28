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

@vertex
fn vs_main(input: VertexInput) -> @builtin(position) vec4<f32> {
  let size = input.instancePosSize.w;
  let worldPos =
    input.instancePosSize.xyz +
    light.cameraRight.xyz * input.localCorner.x * size +
    light.cameraUp.xyz * input.localCorner.y * size;
  return light.viewProj * vec4<f32>(worldPos, 1.0);
}
