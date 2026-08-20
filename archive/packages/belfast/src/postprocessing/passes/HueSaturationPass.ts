import type { Device } from "../../core/Device";
import { ShaderPass } from "../ShaderPass";

const HUE_SATURATION_SHADER = `
@group(0) @binding(0) var passSampler: sampler;
@group(0) @binding(1) var inputTexture: texture_2d<f32>;

struct PassUniforms {
  hue: f32, // in degrees
  saturation: f32,
}
@group(0) @binding(2) var<uniform> uniforms: PassUniforms;

fn rgb2hsv(c: vec3<f32>) -> vec3<f32> {
    let K = vec4<f32>(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
    let p = mix(vec4<f32>(c.bg, K.wz), vec4<f32>(c.gb, K.xy), step(c.b, c.g));
    let q = mix(vec4<f32>(p.xyw, c.r), vec4<f32>(c.r, p.yzx), step(p.x, c.r));

    let d = q.x - min(q.w, q.y);
    let e = 1.0e-10;
    return vec3<f32>(abs(q.z + (q.w - q.y) / (6.0 * d + e)), d / (q.x + e), q.x);
}

fn hsv2rgb(c: vec3<f32>) -> vec3<f32> {
    let K = vec4<f32>(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    let p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
    return c.z * mix(K.xxx, clamp(p - K.xxx, vec3<f32>(0.0), vec3<f32>(1.0)), c.y);
}

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
  let color = textureSample(inputTexture, passSampler, uv);
  var hsv = rgb2hsv(color.rgb);
  
  hsv.x = fract(hsv.x + uniforms.hue / 360.0);
  hsv.y = clamp(hsv.y * uniforms.saturation, 0.0, 1.0);
  
  let rgb = hsv2rgb(hsv);
  return vec4<f32>(rgb, color.a);
}
`;

export function createHueSaturationPass(device: Device): ShaderPass {
  const pass = new ShaderPass(device, HUE_SATURATION_SHADER, {
    label: "HueSaturationPass",
    uniforms: {
      hue: "f32",
      saturation: "f32",
    },
  });

  pass.setUniform("hue", 0.0);
  pass.setUniform("saturation", 1.0);

  return pass;
}
