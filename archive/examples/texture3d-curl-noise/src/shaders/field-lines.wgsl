struct SceneUniforms {
  viewProj: mat4x4<f32>,
  arrowScale: f32,
  visGrid: f32,
  texSize: f32,
  volumeExtent: f32,
}

@group(0) @binding(0) var<uniform> scene: SceneUniforms;
@group(0) @binding(1) var velocityTex: texture_3d<f32>;

struct VertexInput {
  @location(0) localPos: vec3<f32>,
}

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) color: vec3<f32>,
}

@vertex
fn vs_main(input: VertexInput, @builtin(instance_index) instanceIndex: u32) -> VertexOutput {
  let visGrid = u32(scene.visGrid);
  let cellCount = visGrid * visGrid * visGrid;

  var output: VertexOutput;
  if (instanceIndex >= cellCount) {
    output.position = vec4<f32>(0.0, 0.0, -2.0, 1.0);
    output.color = vec3<f32>(0.0);
    return output;
  }

  let iz = instanceIndex / (visGrid * visGrid);
  let rem = instanceIndex % (visGrid * visGrid);
  let iy = rem / visGrid;
  let ix = rem % visGrid;

  let cell = vec3<f32>(f32(ix), f32(iy), f32(iz));
  let uvw = (cell + 0.5) / scene.visGrid;
  let center = (uvw - 0.5) * scene.volumeExtent;

  let texSize = vec3<f32>(textureDimensions(velocityTex));
  let coord = vec3<i32>(uvw * texSize);
  let sample = textureLoad(velocityTex, coord, 0);
  let direction = normalize(sample.xyz);

  // localPos.y is 0 at the cell center, 1 at the arrow tip — fixed length, noise direction only
  let worldPos = center + input.localPos.y * direction * scene.arrowScale;
  output.position = scene.viewProj * vec4<f32>(worldPos, 1.0);
  output.color = abs(direction) * 0.6 + vec3<f32>(0.15);
  return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
  return vec4<f32>(input.color, 1.0);
}
