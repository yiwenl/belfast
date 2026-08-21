export const wgslShadowPcf3x3 = /* wgsl */ `
fn sampleShadowPcf3x3(
  shadowMap: texture_depth_2d,
  shadowSampler: sampler_comparison,
  shadowCoord: vec4<f32>,
  mapSize: f32,
  bias: f32,
) -> f32 {
  let projCoords = shadowCoord.xyz / shadowCoord.w;
  let shadowPos = vec3<f32>(
    projCoords.x * 0.5 + 0.5,
    -projCoords.y * 0.5 + 0.5,
    projCoords.z - bias
  );

  // Out of bounds check without branches affecting textureSampleCompareLevel
  let inBounds = (
    shadowPos.x >= 0.0 && shadowPos.x <= 1.0 &&
    shadowPos.y >= 0.0 && shadowPos.y <= 1.0 &&
    shadowPos.z <= 1.0
  );

  let texelSize = 1.0 / mapSize;
  var shadow: f32 = 0.0;

  for (var y: i32 = -1; y <= 1; y++) {
    for (var x: i32 = -1; x <= 1; x++) {
      let offset = vec2<f32>(f32(x), f32(y)) * texelSize;
      shadow += textureSampleCompareLevel(
        shadowMap,
        shadowSampler,
        shadowPos.xy + offset,
        shadowPos.z
      );
    }
  }

  shadow /= 9.0;
  
  return select(1.0, shadow, inBounds);
}
`;
