import type { Device } from "../core/Device";
import { RenderTarget } from "../core/RenderTarget";
import type { RenderPassOptions } from "../core/RenderPass";

export interface ShadowMapOptions {
  size?: number | [number, number];
  format?: GPUTextureFormat;
  label?: string;
}

export class ShadowMap {
  readonly texture: GPUTexture;
  readonly view: GPUTextureView;
  readonly sampler: GPUSampler;
  readonly size: [number, number];

  private readonly label: string;
  private readonly renderTarget: RenderTarget;

  private constructor(device: Device, options: ShadowMapOptions = {}) {
    this.label = options.label ?? "ShadowMap";
    let width = 1024;
    let height = 1024;
    if (typeof options.size === "number") {
      width = options.size;
      height = options.size;
    } else if (Array.isArray(options.size)) {
      width = options.size[0];
      height = options.size[1];
    }
    this.size = [width, height];

    this.renderTarget = RenderTarget.create(device, {
      label: `${this.label}Target`,
      width,
      height,
      withDepth: true,
      depthFormat: options.format ?? "depth32float",
      depthTextureUsage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING,
    });

    if (!this.renderTarget.depthTexture || !this.renderTarget.depthView) {
      throw new Error("Failed to create shadow map depth texture");
    }

    this.texture = this.renderTarget.depthTexture;
    this.view = this.renderTarget.depthView;

    this.sampler = device.gpu.createSampler({
      label: `${this.label}Sampler`,
      compare: "less",
      magFilter: "linear",
      minFilter: "linear",
      addressModeU: "clamp-to-edge",
      addressModeV: "clamp-to-edge",
    });
  }

  static create(device: Device, options?: ShadowMapOptions): ShadowMap {
    return new ShadowMap(device, options);
  }

  beginRenderPass(
    commandEncoder: GPUCommandEncoder,
    options: Omit<RenderPassOptions, "colorAttachments"> = {},
  ): GPURenderPassEncoder {
    const depthStencilAttachment = options.depthStencilAttachment ?? {
      view: this.view,
      depthClearValue: 1.0,
      depthLoadOp: "clear",
      depthStoreOp: "store",
    };

    return commandEncoder.beginRenderPass({
      label: `${this.label}Pass`,
      colorAttachments: [],
      depthStencilAttachment,
      ...options,
    });
  }

  destroy(): void {
    this.renderTarget.destroy();
  }
}
