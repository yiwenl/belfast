// Pass 1 of the GPU sort: seed one sort key per plane.
//
// Writes `keys[i] = { dist, index }` where `dist` is the squared distance from
// the plane center to the camera (squared is monotonic, so it orders the same
// as true distance but skips the sqrt). Padding slots beyond `count` (the array
// is padded up to a power of two for bitonic sort) get `dist = -1` so they sort
// to the very back of a descending order and are never drawn.

struct Plane {
  posSize: vec4<f32>, // xyz = world center, w = half-size
  color: vec4<f32>,   // rgb + alpha
}

struct Key {
  dist: f32,
  index: u32,
}

struct DistParams {
  cameraPos: vec4<f32>,
  count: u32, // number of real planes
  total: u32, // padded power-of-two length of `keys`
  pad0: u32,
  pad1: u32,
}

@group(0) @binding(0) var<uniform> params: DistParams;
@group(0) @binding(1) var<storage, read> planes: array<Plane>;
@group(0) @binding(2) var<storage, read_write> keys: array<Key>;

@compute @workgroup_size(256)
fn cs_main(@builtin(global_invocation_id) globalId: vec3<u32>) {
  let i = globalId.x;
  if (i >= params.total) {
    return;
  }

  if (i < params.count) {
    let d = planes[i].posSize.xyz - params.cameraPos.xyz;
    keys[i] = Key(dot(d, d), i);
  } else {
    keys[i] = Key(-1.0, i);
  }
}
