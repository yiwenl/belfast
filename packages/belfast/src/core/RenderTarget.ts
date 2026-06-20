import type { Device } from "./Device";
import { beginRenderPass, type RenderPassOptions } from "./RenderPass";

export interface RenderTargetOptions {
  width: number;
  height: number;
  label?: string;
  format?: GPUTextureFormat;
  sampleCount?: number;
  withDepth?: boolean;
  depthFormat?: GPUTextureFormat;
  depthTextureUsage?: GPUTextureUsageFlags;
}

export class RenderTarget {
  readonly format: GPUTextureFormat;
  readonly depthFormat?: GPUTextureFormat;
  readonly sampler: GPUSampler;

  private readonly device: Device;
  private readonly label: string;
  private readonly withDepth: boolean;
  private readonly depthTextureUsage: GPUTextureUsageFlags;
  private readonly sampleCount: number;
  private colorTexture: GPUTexture;
  private multisampledColorTexture: GPUTexture | null = null;
  private depthTextureInternal: GPUTexture | null = null;
  private colorViewInternal: GPUTextureView;
  private multisampledColorViewInternal: GPUTextureView | null = null;
  private depthViewInternal: GPUTextureView | undefined;
  private widthInternal: number;
  private heightInternal: number;

  private constructor(device: Device, options: RenderTargetOptions) {
    this.device = device;
    this.label = options.label ?? "RenderTarget";
    this.format = options.format ?? (device.hdr ? "rgba16float" : device.format);
    this.sampleCount = options.sampleCount ?? 1;
    this.withDepth = options.withDepth ?? false;
    this.depthTextureUsage = options.depthTextureUsage ?? GPUTextureUsage.RENDER_ATTACHMENT;
    this.depthFormat = this.withDepth ? (options.depthFormat ?? "depth24plus") : undefined;
    this.widthInternal = Math.max(1, Math.floor(options.width));
    this.heightInternal = Math.max(1, Math.floor(options.height));

    this.sampler = device.gpu.createSampler({
      label: `${this.label}Sampler`,
      addressModeU: "clamp-to-edge",
      addressModeV: "clamp-to-edge",
      magFilter: "linear",
      minFilter: "linear",
    });

    const {
      colorTexture,
      colorView,
      multisampledColorTexture,
      multisampledColorView,
      depthTexture,
      depthView,
    } = this.createTextures();
    this.colorTexture = colorTexture;
    this.colorViewInternal = colorView;
    this.multisampledColorTexture = multisampledColorTexture;
    this.multisampledColorViewInternal = multisampledColorView;
    this.depthTextureInternal = depthTexture;
    this.depthViewInternal = depthView;
  }

  static create(device: Device, options: RenderTargetOptions): RenderTarget {
    return new RenderTarget(device, options);
  }

  get width(): number {
    return this.widthInternal;
  }

  get height(): number {
    return this.heightInternal;
  }

  get colorView(): GPUTextureView {
    return this.colorViewInternal;
  }

  get depthView(): GPUTextureView | undefined {
    return this.depthViewInternal;
  }

  get depthTexture(): GPUTexture | undefined {
    return this.depthTextureInternal ?? undefined;
  }

  resize(width: number, height: number): void {
    const w = Math.max(1, Math.floor(width));
    const h = Math.max(1, Math.floor(height));
    if (w === this.widthInternal && h === this.heightInternal) {
      return;
    }
    this.widthInternal = w;
    this.heightInternal = h;

    this.colorTexture.destroy();
    this.multisampledColorTexture?.destroy();
    this.depthTextureInternal?.destroy();

    const {
      colorTexture,
      colorView,
      multisampledColorTexture,
      multisampledColorView,
      depthTexture,
      depthView,
    } = this.createTextures();
    this.colorTexture = colorTexture;
    this.colorViewInternal = colorView;
    this.multisampledColorTexture = multisampledColorTexture;
    this.multisampledColorViewInternal = multisampledColorView;
    this.depthTextureInternal = depthTexture;
    this.depthViewInternal = depthView;
  }

  beginRenderPass(
    commandEncoder: GPUCommandEncoder,
    options: RenderPassOptions = {},
  ): GPURenderPassEncoder {
    const finalOptions = { ...options };
    if (this.sampleCount > 1 && this.multisampledColorViewInternal) {
      return beginRenderPass(commandEncoder, this.multisampledColorViewInternal, {
        ...finalOptions,
        resolveTarget: this.colorViewInternal,
        storeOp: "discard", // We only want to store the resolved result
      });
    }
    return beginRenderPass(commandEncoder, this, finalOptions);
  }

  destroy(): void {
    this.colorTexture.destroy();
    this.multisampledColorTexture?.destroy();
    this.depthTextureInternal?.destroy();
  }

  private createTextures(): {
    colorTexture: GPUTexture;
    colorView: GPUTextureView;
    multisampledColorTexture: GPUTexture | null;
    multisampledColorView: GPUTextureView | null;
    depthTexture: GPUTexture | null;
    depthView: GPUTextureView | undefined;
  } {
    const colorTexture = this.device.gpu.createTexture({
      label: `${this.label}ColorTexture`,
      size: [this.widthInternal, this.heightInternal],
      format: this.format,
      usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING,
    });
    const colorView = colorTexture.createView({ label: `${this.label}ColorView` });

    let multisampledColorTexture: GPUTexture | null = null;
    let multisampledColorView: GPUTextureView | null = null;

    if (this.sampleCount > 1) {
      multisampledColorTexture = this.device.gpu.createTexture({
        label: `${this.label}MultisampledColorTexture`,
        size: [this.widthInternal, this.heightInternal],
        format: this.format,
        sampleCount: this.sampleCount,
        usage: GPUTextureUsage.RENDER_ATTACHMENT,
      });
      multisampledColorView = multisampledColorTexture.createView({
        label: `${this.label}MultisampledColorView`,
      });
    }

    if (!this.withDepth || !this.depthFormat) {
      return {
        colorTexture,
        colorView,
        multisampledColorTexture,
        multisampledColorView,
        depthTexture: null,
        depthView: undefined,
      };
    }

    const depthTexture = this.device.gpu.createTexture({
      label: `${this.label}DepthTexture`,
      size: [this.widthInternal, this.heightInternal],
      format: this.depthFormat,
      sampleCount: this.sampleCount,
      usage: this.depthTextureUsage,
    });
    const depthView = depthTexture.createView({ label: `${this.label}DepthView` });
    return {
      colorTexture,
      colorView,
      multisampledColorTexture,
      multisampledColorView,
      depthTexture,
      depthView,
    };
  }
}
