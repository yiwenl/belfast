struct CameraUniforms {
  view: mat4x4<f32>,
  projection: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;

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
  output.position = camera.projection * camera.view * vec4f(position, 1.0);
  output.color = color;
  return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4f {
  return vec4f(input.color, 1.0);
}
