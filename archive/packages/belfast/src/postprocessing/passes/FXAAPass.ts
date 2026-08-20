import type { Device } from "../../core/Device";
import { ShaderPass } from "../ShaderPass";

const FXAA_SHADER = `
@group(0) @binding(0) var passSampler: sampler;
@group(0) @binding(1) var inputTexture: texture_2d<f32>;

struct PassUniforms {
  resolution: vec2<f32>,
}
@group(0) @binding(2) var<uniform> uniforms: PassUniforms;

const FXAA_SPAN_MAX: f32 = 8.0;
const FXAA_REDUCE_MUL: f32 = 1.0 / 8.0;
const FXAA_REDUCE_MIN: f32 = 1.0 / 128.0;

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
  let texelSize = 1.0 / uniforms.resolution;
  
  let rgbNW = textureSample(inputTexture, passSampler, uv + vec2<f32>(-1.0, -1.0) * texelSize).rgb;
  let rgbNE = textureSample(inputTexture, passSampler, uv + vec2<f32>(1.0, -1.0) * texelSize).rgb;
  let rgbSW = textureSample(inputTexture, passSampler, uv + vec2<f32>(-1.0, 1.0) * texelSize).rgb;
  let rgbSE = textureSample(inputTexture, passSampler, uv + vec2<f32>(1.0, 1.0) * texelSize).rgb;
  let rgbM  = textureSample(inputTexture, passSampler, uv).rgb;
  
  let luma = vec3<f32>(0.299, 0.587, 0.114);
  let lumaNW = dot(rgbNW, luma);
  let lumaNE = dot(rgbNE, luma);
  let lumaSW = dot(rgbSW, luma);
  let lumaSE = dot(rgbSE, luma);
  let lumaM  = dot(rgbM,  luma);
  
  let lumaMin = min(lumaM, min(min(lumaNW, lumaNE), min(lumaSW, lumaSE)));
  let lumaMax = max(lumaM, max(max(lumaNW, lumaNE), max(lumaSW, lumaSE)));
  
  var dir: vec2<f32>;
  dir.x = -((lumaNW + lumaNE) - (lumaSW + lumaSE));
  dir.y =  ((lumaNW + lumaSW) - (lumaNE + lumaSE));
  
  let dirReduce = max((lumaNW + lumaNE + lumaSW + lumaSE) * (0.25 * FXAA_REDUCE_MUL), FXAA_REDUCE_MIN);
  let rcpDirMin = 1.0 / (min(abs(dir.x), abs(dir.y)) + dirReduce);
  
  dir = min(vec2<f32>(FXAA_SPAN_MAX, FXAA_SPAN_MAX),
            max(vec2<f32>(-FXAA_SPAN_MAX, -FXAA_SPAN_MAX),
            dir * rcpDirMin)) * texelSize;
            
  let rgbA = 0.5 * (
      textureSample(inputTexture, passSampler, uv + dir * (1.0/3.0 - 0.5)).rgb +
      textureSample(inputTexture, passSampler, uv + dir * (2.0/3.0 - 0.5)).rgb
  );
  
  let rgbB = rgbA * 0.5 + 0.25 * (
      textureSample(inputTexture, passSampler, uv + dir * (0.0/3.0 - 0.5)).rgb +
      textureSample(inputTexture, passSampler, uv + dir * (3.0/3.0 - 0.5)).rgb
  );
  
  let lumaB = dot(rgbB, luma);
  let a = textureSample(inputTexture, passSampler, uv).a;
  
  if (lumaB < lumaMin || lumaB > lumaMax) {
      return vec4<f32>(rgbA, a);
  } else {
      return vec4<f32>(rgbB, a);
  }
}
`;

export function createFXAAPass(device: Device): ShaderPass {
  return new ShaderPass(device, FXAA_SHADER, {
    label: "FXAAPass",
    uniforms: {
      resolution: "vec2f",
    },
  });
}
