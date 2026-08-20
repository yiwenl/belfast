struct SceneUniforms {
  viewProj: mat4x4<f32>,
  model: mat4x4<f32>,
  lightDir: vec4<f32>,
}

@group(0) @binding(0) var<uniform> scene: SceneUniforms;

struct VertexInput {
  @location(0) position: vec3<f32>,
  @location(1) normal: vec3<f32>,
}

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) normal: vec3<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
  var output: VertexOutput;
  let worldPos = scene.model * vec4<f32>(input.position, 1.0);
  output.position = scene.viewProj * worldPos;
  output.normal = normalize((scene.model * vec4<f32>(input.normal, 0.0)).xyz);
  return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
  let n = normalize(input.normal);
  let l = normalize(-scene.lightDir.xyz);
  let diffuse = max(dot(n, l), 0.0);
  let baseColor = vec3<f32>(0.95, 0.55, 0.3);
  let color = baseColor * (0.2 + 0.8 * diffuse);
  return vec4<f32>(color, 1.0);
}
