// Instanced billboard planes rendered in sorted order.
//
// Geometry is a single unit quad (6 vertices). Per-instance data lives in a
// storage buffer; the sorted index buffer (`keys`) maps draw order -> plane,
// so instance 0 is the farthest plane and the last instance is the nearest.
// Combined with src-alpha / one-minus-src-alpha blending this yields correct
// back-to-front transparency.

struct SceneUniforms {
  viewProj: mat4x4<f32>,
  cameraRight: vec4<f32>,
  cameraUp: vec4<f32>,
}

struct Plane {
  posSize: vec4<f32>, // xyz = world center, w = half-size
  color: vec4<f32>,   // rgb + alpha
}

struct Key {
  dist: f32,
  index: u32,
}

@group(0) @binding(0) var<uniform> scene: SceneUniforms;
@group(1) @binding(0) var<storage, read> planes: array<Plane>;
@group(1) @binding(1) var<storage, read> keys: array<Key>;

struct VertexInput {
  @location(0) localPosition: vec3<f32>,
  @builtin(instance_index) instance: u32,
}

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
  let planeId = keys[input.instance].index;
  let plane = planes[planeId];
  let size = plane.posSize.w;

  let worldPos =
    plane.posSize.xyz +
    scene.cameraRight.xyz * input.localPosition.x * size +
    scene.cameraUp.xyz * input.localPosition.y * size;

  var output: VertexOutput;
  output.position = scene.viewProj * vec4<f32>(worldPos, 1.0);
  output.color = plane.color;
  return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
  return input.color;
}
