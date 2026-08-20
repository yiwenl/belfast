import type { Device } from "../../core/Device";
import { ShaderPass } from "../ShaderPass";

const CURVE_SHADER = `
@group(0) @binding(0) var passSampler: sampler;
@group(0) @binding(1) var inputTexture: texture_2d<f32>;

struct PassUniforms {
  x1: f32,
  y1: f32,
  x2: f32,
  y2: f32,
}
@group(0) @binding(2) var<uniform> uniforms: PassUniforms;

fn get_bezier_y(x: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    if (x <= 0.0) { return 0.0; }
    if (x >= 1.0) { return 1.0; }
    
    var min_t: f32 = 0.0;
    var max_t: f32 = 1.0;
    var t: f32 = 0.5;
    for (var i: i32 = 0; i < 12; i = i + 1) {
        let one_minus_t = 1.0 - t;
        let b_x = 3.0 * one_minus_t * one_minus_t * t * x1 + 3.0 * one_minus_t * t * t * x2 + t * t * t;
        if (b_x < x) {
            min_t = t;
        } else {
            max_t = t;
        }
        t = (max_t + min_t) * 0.5;
    }
    let one_minus_t = 1.0 - t;
    return 3.0 * one_minus_t * one_minus_t * t * y1 + 3.0 * one_minus_t * t * t * y2 + t * t * t;
}

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
  let color = textureSample(inputTexture, passSampler, uv);
  
  let r = get_bezier_y(color.r, uniforms.x1, uniforms.y1, uniforms.x2, uniforms.y2);
  let g = get_bezier_y(color.g, uniforms.x1, uniforms.y1, uniforms.x2, uniforms.y2);
  let b = get_bezier_y(color.b, uniforms.x1, uniforms.y1, uniforms.x2, uniforms.y2);
  
  return vec4<f32>(r, g, b, color.a);
}
`;

export function createCurvePass(device: Device): ShaderPass {
  const pass = new ShaderPass(device, CURVE_SHADER, {
    label: "CurvePass",
    uniforms: {
      x1: "f32",
      y1: "f32",
      x2: "f32",
      y2: "f32",
    },
  });

  // Linear by default
  pass.setUniform("x1", 0.33);
  pass.setUniform("y1", 0.33);
  pass.setUniform("x2", 0.66);
  pass.setUniform("y2", 0.66);

  return pass;
}
