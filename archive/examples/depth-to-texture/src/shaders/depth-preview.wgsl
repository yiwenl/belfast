@group(0) @binding(0) var depthTexture: texture_depth_2d;
@group(0) @binding(1) var<uniform> previewParams: vec4<f32>;

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertexIndex: u32) -> VertexOutput {
  var output: VertexOutput;
  var positions = array<vec2<f32>, 3>(
    vec2<f32>(-1.0, -1.0),
    vec2<f32>(3.0, -1.0),
    vec2<f32>(-1.0, 3.0),
  );
  let pos = positions[vertexIndex];
  output.position = vec4<f32>(pos, 0.0, 1.0);
  output.uv = vec2<f32>(pos.x * 0.5 + 0.5, -pos.y * 0.5 + 0.5);
  return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
  let dims = textureDimensions(depthTexture);
  let uv = clamp(input.uv, vec2<f32>(0.0), vec2<f32>(0.99999));
  let coord = vec2<i32>(uv * vec2<f32>(dims));
  let d = textureLoad(depthTexture, coord, 0);
  let near = previewParams.x;
  let far = previewParams.y;
  let rangeNear = previewParams.z;
  let rangeFar = previewParams.w;
  let zNdc = d * 2.0 - 1.0;
  let linearDepth = (2.0 * near * far) / (far + near - zNdc * (far - near));
  let t = clamp((linearDepth - rangeNear) / max(1e-5, rangeFar - rangeNear), 0.0, 1.0);
  let gray = 1.0 - t;
  return vec4<f32>(vec3<f32>(gray), 1.0);
}
