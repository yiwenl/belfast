import type { Device } from "../../core/Device";
import { ShaderPass } from "../ShaderPass";

const VIGNETTE_SHADER = `
@group(0) @binding(0) var passSampler: sampler;
@group(0) @binding(1) var inputTexture: texture_2d<f32>;

struct PassUniforms {
  radius: f32,
  strength: f32,
}
@group(0) @binding(2) var<uniform> uniforms: PassUniforms;

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
  let color = textureSample(inputTexture, passSampler, uv);
  
  // distance from center
  let dist = distance(uv, vec2<f32>(0.5, 0.5));
  let vignette = smoothstep(uniforms.radius, uniforms.radius - uniforms.strength, dist);
  
  return vec4<f32>(color.rgb * vignette, color.a);
}
`;

export function createVignettePass(device: Device): ShaderPass {
  const pass = new ShaderPass(device, VIGNETTE_SHADER, {
    label: "VignettePass",
    uniforms: {
      radius: "f32",
      strength: "f32",
    },
  });

  // Set defaults
  pass.setUniform("radius", 0.75);
  pass.setUniform("strength", 0.4);

  return pass;
}
