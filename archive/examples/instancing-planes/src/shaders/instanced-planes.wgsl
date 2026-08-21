struct SceneUniforms {
  viewProj: mat4x4<f32>,
  cameraRight: vec4<f32>,
  cameraUp: vec4<f32>,
}

@group(0) @binding(0) var<uniform> scene: SceneUniforms;

struct VertexInput {
  @location(0) localPosition: vec3<f32>,
  @location(1) instancePosSize: vec4<f32>, // xyz + size
  @location(2) instanceColor: vec4<f32>, // rgb + alpha
}

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
  var output: VertexOutput;
  let size = input.instancePosSize.w;
  let worldPos =
    input.instancePosSize.xyz +
    scene.cameraRight.xyz * input.localPosition.x * size +
    scene.cameraUp.xyz * input.localPosition.y * size;
  output.position = scene.viewProj * vec4<f32>(worldPos, 1.0);
  output.color = input.instanceColor;
  return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
  return input.color;
}
