import type { Device } from "../../core/Device";
import { ShaderPass } from "../ShaderPass";

const CONTRAST_BRIGHTNESS_SHADER = `
@group(0) @binding(0) var passSampler: sampler;
@group(0) @binding(1) var inputTexture: texture_2d<f32>;

struct PassUniforms {
  contrast: f32,
  brightness: f32,
}
@group(0) @binding(2) var<uniform> uniforms: PassUniforms;

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
  let color = textureSample(inputTexture, passSampler, uv);
  
  var rgb = color.rgb;
  rgb = (rgb - 0.5) * uniforms.contrast + 0.5;
  rgb = rgb * uniforms.brightness;
  
  return vec4<f32>(rgb, color.a);
}
`;

export function createContrastBrightnessPass(device: Device): ShaderPass {
  const pass = new ShaderPass(device, CONTRAST_BRIGHTNESS_SHADER, {
    label: "ContrastBrightnessPass",
    uniforms: {
      contrast: "f32",
      brightness: "f32",
    },
  });

  pass.setUniform("contrast", 1.0);
  pass.setUniform("brightness", 1.0);

  return pass;
}
