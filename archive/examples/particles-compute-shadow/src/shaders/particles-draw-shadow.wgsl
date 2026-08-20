struct SceneUniforms {
  viewProj: mat4x4<f32>,
  cameraRight: vec4<f32>,
  cameraUp: vec4<f32>,
  lightViewProj: mat4x4<f32>,
  lightDir: vec4<f32>, // world-space direction toward the light
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

// PCFShadow removed, sampleShadowPcf3x3 will be injected from TS

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
  // Spherical impostor: treat the disc as a sphere cross-section.
  let n2 = (input.discUv - vec2(0.5)) * 2.0; // [-1, 1] across the disc
  let r2 = dot(n2, n2);
  if (r2 > 1.0) {
    discard;
  }

  // Reconstruct a world-space normal that bulges toward the camera.
  let nz = sqrt(1.0 - r2);
  let forward = normalize(cross(scene.cameraRight.xyz, scene.cameraUp.xyz));
  let normal = normalize(
    scene.cameraRight.xyz * n2.x + scene.cameraUp.xyz * n2.y + forward * nz,
  );

  let L = normalize(scene.lightDir.xyz);
  let diffuse = max(dot(normal, L), 0.0) * 2.0;
  let shadow = sampleShadowPcf3x3(shadowMap, shadowSampler, input.shadowCoord, 2048.0, 0.001);

  let ambient = 0.25;
  let lit = ambient + (1.0 - ambient) * diffuse * shadow;

  let soft = 1.0 - smoothstep(0.5, 1.0, sqrt(r2));
  let rgb = input.color.rgb * lit;
  return vec4(rgb, input.color.a * soft);
}
