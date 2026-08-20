@group(0) @binding(0) var hdrTexture: texture_2d<f32>;
@group(0) @binding(1) var hdrSampler: sampler;

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

// Rec.709 scene-referred linear → sRGB-linear for extended canvas tone mapping.
const REC709_LUMA: vec3<f32> = vec3(0.2126, 0.7152, 0.0722);
const HDR_HEADROOM: f32 = 8.0;

fn presentHdr(c: vec3<f32>) -> vec3<f32> {
  let x = max(c, vec3<f32>(0.0));
  let luma = dot(x, REC709_LUMA);
  let mappedLuma = (luma / (1.0 + luma)) * HDR_HEADROOM;
  return x * (mappedLuma / max(luma, 1e-4));
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
  let hdr = textureSample(hdrTexture, hdrSampler, input.uv).rgb;
  let adjusted = presentHdr(hdr);
  let mapped = pow(adjusted, vec3<f32>(1.0 / 2.2));
  return vec4<f32>(mapped, 1.0);
  // return vec4<f32>(presentHdr(hdr), 1.0);
}
