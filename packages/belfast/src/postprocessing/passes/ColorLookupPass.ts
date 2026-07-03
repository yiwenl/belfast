import type { Device } from "../../core/Device";
import { ShaderPass } from "../ShaderPass";
import { BindGroup } from "../../core/BindGroup";

const LUT_SHADER = `
@group(0) @binding(0) var passSampler: sampler;
@group(0) @binding(1) var inputTexture: texture_2d<f32>;

struct PassUniforms {
  strength: f32,
  flipY: f32,
}
@group(0) @binding(2) var<uniform> uniforms: PassUniforms;

@group(0) @binding(3) var lutSampler: sampler;
@group(0) @binding(4) var lutTexture: texture_2d<f32>;

fn lookup(textureColor: vec4<f32>) -> vec4<f32> {
    var color = clamp(textureColor, vec4<f32>(0.0), vec4<f32>(1.0));

    let blueColor = color.b * 63.0;

    var quad1: vec2<f32>;
    quad1.y = floor(floor(blueColor) / 8.0);
    quad1.x = floor(blueColor) - (quad1.y * 8.0);

    var quad2: vec2<f32>;
    quad2.y = floor(ceil(blueColor) / 8.0);
    quad2.x = ceil(blueColor) - (quad2.y * 8.0);

    var texPos1: vec2<f32>;
    texPos1.x = (quad1.x * 0.125) + 0.5/512.0 + ((0.125 - 1.0/512.0) * color.r);
    texPos1.y = (quad1.y * 0.125) + 0.5/512.0 + ((0.125 - 1.0/512.0) * color.g);
    
    // WebGPU textures are 0,0 at top-left, same as HTML Image
    // But depending on the LUT orientation we might need to flip Y.
    if (uniforms.flipY > 0.5) {
      texPos1.y = 1.0 - texPos1.y;
    }

    var texPos2: vec2<f32>;
    texPos2.x = (quad2.x * 0.125) + 0.5/512.0 + ((0.125 - 1.0/512.0) * color.r);
    texPos2.y = (quad2.y * 0.125) + 0.5/512.0 + ((0.125 - 1.0/512.0) * color.g);

    if (uniforms.flipY > 0.5) {
      texPos2.y = 1.0 - texPos2.y;
    }

    let newColor1 = textureSample(lutTexture, lutSampler, texPos1);
    let newColor2 = textureSample(lutTexture, lutSampler, texPos2);

    let newColor = mix(newColor1, newColor2, fract(blueColor));
    return mix(textureColor, vec4<f32>(newColor.rgb, textureColor.a), uniforms.strength);
}

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
  let color = textureSample(inputTexture, passSampler, uv);
  return lookup(color);
}
`;

export class ColorLookupPass extends ShaderPass {
  private lutTexture: GPUTexture | null = null;
  private lutView: GPUTextureView | null = null;
  private lutSampler: GPUSampler;

  private currentUrl: string | null = null;
  private currentInputViewInternal?: GPUTextureView;
  private currentBindGroupInternal?: BindGroup;

  constructor(device: Device) {
    super(device, LUT_SHADER, {
      label: "ColorLookupPass",
      uniforms: {
        strength: "f32",
        flipY: "f32",
      },
    });

    this.lutSampler = device.gpu.createSampler({
      label: "ColorLookupSampler",
      addressModeU: "clamp-to-edge",
      addressModeV: "clamp-to-edge",
      magFilter: "linear",
      minFilter: "linear",
    });
  }

  async loadLUT(url: string) {
    if (this.currentUrl === url) return;
    this.currentUrl = url;

    try {
      const response = await fetch(url);
      const blob = await response.blob();
      const imageBitmap = await createImageBitmap(blob);

      // Ensure we haven't started loading a different URL while waiting
      if (this.currentUrl !== url) return;

      this.setLUT(imageBitmap);
    } catch (e) {
      console.error("Failed to load LUT", e);
    }
  }

  setLUT(imageBitmap: ImageBitmap | HTMLCanvasElement | HTMLImageElement) {
    if (this.lutTexture) {
      this.lutTexture.destroy();
    }

    this.lutTexture = this.device.gpu.createTexture({
      label: "ColorLookupTexture",
      size: [imageBitmap.width, imageBitmap.height, 1],
      format: "rgba8unorm",
      usage:
        GPUTextureUsage.TEXTURE_BINDING |
        GPUTextureUsage.COPY_DST |
        GPUTextureUsage.RENDER_ATTACHMENT,
    });
    this.lutView = this.lutTexture.createView();

    this.device.gpu.queue.copyExternalImageToTexture(
      { source: imageBitmap },
      { texture: this.lutTexture },
      [imageBitmap.width, imageBitmap.height],
    );

    // Invalidate bind group to trigger recreation
    this.currentBindGroupInternal = undefined;
  }

  override render(pass: GPURenderPassEncoder, inputView: GPUTextureView) {
    if (!this.lutView) {
      // If LUT isn't loaded, just render input texture as is? Or skip?
      // Wait, we can't skip `this.draw.draw` because the pipeline expects 5 bindings.
      // If we don't bind a texture, we get a validation error.
      // So if no LUT is loaded, we shouldn't draw? Or we draw nothing?
      // But EffectComposer renders over the ping pong buffer, if we draw nothing it's just black.
      // Alternatively, we can bind `inputView` as a dummy LUT temporarily.
    }

    if (this.uniforms && this.uniformBuffer) {
      this.uniforms.writeToBuffer(this.uniformBuffer, this.device);
    }

    if (this.currentInputViewInternal !== inputView || !this.currentBindGroupInternal) {
      this.currentInputViewInternal = inputView;

      const entries: { binding: number; resource: GPUBindingResource }[] = [
        { binding: 0, resource: this.sampler },
        { binding: 1, resource: inputView },
      ];
      if (this.uniformBuffer) {
        entries.push({ binding: 2, resource: { buffer: this.uniformBuffer.gpu } });
      }

      entries.push({ binding: 3, resource: this.lutSampler });
      entries.push({ binding: 4, resource: this.lutView || inputView }); // fallback to inputView to prevent crashes

      this.currentBindGroupInternal = BindGroup.create(
        this.device,
        this.bindGroupLayout,
        entries,
        "ColorLookupBindGroup",
      );
    }

    this.draw.draw(pass, 3, this.currentBindGroupInternal);
  }

  override destroy() {
    super.destroy();
    if (this.lutTexture) {
      this.lutTexture.destroy();
    }
  }
}

export function createColorLookupPass(device: Device): ColorLookupPass {
  return new ColorLookupPass(device);
}
