struct SceneUniforms {
  viewProj: mat4x4<f32>,
  cameraRight: vec4<f32>,
  cameraUp: vec4<f32>,
}

@group(0) @binding(0) var<uniform> scene: SceneUniforms;

struct VertexInput {
  @location(0) localCorner: vec4<f32>, // xy = offset, zw = disc UV
  @location(1) instancePosSize: vec4<f32>,
  @location(2) instanceColor: vec4<f32>,
}

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) color: vec4<f32>,
  @location(1) discUv: vec2<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
  var output: VertexOutput;
  let size = input.instancePosSize.w;
  let worldPos =
    input.instancePosSize.xyz +
    scene.cameraRight.xyz * input.localCorner.x * size +
    scene.cameraUp.xyz * input.localCorner.y * size;
  output.position = scene.viewProj * vec4<f32>(worldPos, 1.0);
  output.color = input.instanceColor;
  output.discUv = input.localCorner.zw;
  return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
  let dist = length(input.discUv - vec2(0.5));
  if (dist > 0.5) {
    discard;
  }
  let soft = 1.0 - smoothstep(0.35, 0.5, dist);
  return vec4(input.color.rgb * soft, input.color.a * soft);
}
