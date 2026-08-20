struct SceneUniforms {
  viewProj: mat4x4<f32>,
  time: f32,
}

@group(0) @binding(0) var<uniform> scene: SceneUniforms;

struct VertexOutput {
  @builtin(position) position: vec4f,
  @location(0) color: vec3f,
}

@vertex
fn vs_main(
  @location(0) position: vec3f,
  @location(1) color: vec3f,
) -> VertexOutput {
  var output: VertexOutput;
  let scale = 0.75 + 0.25 * sin(scene.time);
  output.position = scene.viewProj * vec4f(position * scale, 1.0);
  output.color = color;
  return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4f {
  return vec4f(input.color, 1.0);
}
