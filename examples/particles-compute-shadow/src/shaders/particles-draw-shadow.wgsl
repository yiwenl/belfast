struct SceneUniforms {
  viewProj: mat4x4<f32>,
  cameraRight: vec4<f32>,
  cameraUp: vec4<f32>,
  lightViewProj: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> scene: SceneUniforms;
@group(0) @binding(1) var shadowMap: texture_depth_2d;
@group(0) @binding(2) var shadowSampler: sampler_comparison;

struct VertexInput {
  @location(0) localCorner: vec4<f32>,
  @location(1) instancePosSize: vec4<f32>,
  @location(2) instanceColor: vec4<f32>,
}

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) color: vec4<f32>,
  @location(1) discUv: vec2<f32>,
  @location(2) shadowCoord: vec4<f32>,
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
  output.shadowCoord = scene.lightViewProj * vec4<f32>(worldPos, 1.0);
  return output;
}

fn PCFShadow(shadowCoord: vec4<f32>) -> f32 {
  let shadowPos = shadowCoord.xyz / shadowCoord.w;
  let shadowUv = shadowPos.xy * 0.5 + 0.5;
  var shadow = 0.0;
  let size = vec2<f32>(textureDimensions(shadowMap, 0));
  let texelSize = 2.0 / size;

  for (var x = -1; x <= 1; x++) {
    for (var y = -1; y <= 1; y++) {
      let pcfDepth = textureSampleCompare(
        shadowMap,
        shadowSampler,
        shadowUv + vec2<f32>(f32(x), f32(y)) * texelSize,
        shadowPos.z - 0.001,
      );
      shadow += pcfDepth;
    }
  }
  return shadow / 9.0;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
  let dist = length(input.discUv - vec2(0.5));
  if (dist > 0.5) {
    discard;
  }

  let soft = 1.0 - smoothstep(0.35, 0.5, dist);
  let shadow = PCFShadow(input.shadowCoord);

  let lighting = mix(0.25, 1.0, shadow);
  let rgb = input.color.rgb * soft * lighting;
  return vec4(rgb, input.color.a * soft);
}
