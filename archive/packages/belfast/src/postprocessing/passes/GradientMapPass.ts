import type { Device } from "../../core/Device";
import { ShaderPass } from "../ShaderPass";

const GRADIENT_MAP_SHADER = `
@group(0) @binding(0) var passSampler: sampler;
@group(0) @binding(1) var inputTexture: texture_2d<f32>;

struct PassUniforms {
  color1: vec3<f32>,
  color2: vec3<f32>,
}
@group(0) @binding(2) var<uniform> uniforms: PassUniforms;

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
  let color = textureSample(inputTexture, passSampler, uv);
  
  let luminance = dot(color.rgb, vec3<f32>(0.299, 0.587, 0.114));
  let mapped = mix(uniforms.color1, uniforms.color2, luminance);
  
  return vec4<f32>(mapped, color.a);
}
`;

export function createGradientMapPass(device: Device): ShaderPass {
  const pass = new ShaderPass(device, GRADIENT_MAP_SHADER, {
    label: "GradientMapPass",
    uniforms: {
      color1: "vec3f",
      color2: "vec3f",
    },
  });

  pass.setUniform("color1", [0.0, 0.0, 0.0]);
  pass.setUniform("color2", [1.0, 1.0, 1.0]);

  return pass;
}
