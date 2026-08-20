struct SceneUniforms {
  viewProj: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> scene: SceneUniforms;
@group(0) @binding(1) var envMap: texture_2d<f32>;
@group(0) @binding(2) var envSampler: sampler;

struct VertexInput {
  @location(0) position: vec3<f32>,
}

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) worldDir: vec3<f32>,
}

fn dirToEquirectUV(dir: vec3<f32>) -> vec2<f32> {
  let d = normalize(dir);
  let u = atan2(d.z, d.x) / (2.0 * 3.14159265) + 0.5;
  let v = 1.0 - (asin(clamp(d.y, -1.0, 1.0)) / 3.14159265 + 0.5);
  return vec2<f32>(u, v);
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
  var output: VertexOutput;
  output.worldDir = input.position;
  var clipPos = scene.viewProj * vec4<f32>(input.position, 1.0);
  // Pin skybox to the far plane so it always fills the background.
  output.position = vec4<f32>(clipPos.xy, clipPos.w, clipPos.w);
  return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
  let uv = dirToEquirectUV(input.worldDir);
  let hdr = textureSample(envMap, envSampler, uv).rgb;
  return vec4<f32>(hdr, 1.0);
}
