import type { Device } from "./Device";

export interface Texture2DOptions {
  label?: string;
  format?: GPUTextureFormat;
  /** Override default `TEXTURE_BINDING | STORAGE_BINDING` when needed. */
  usage?: GPUTextureUsageFlags;
  addressModeU?: GPUAddressMode;
  addressModeV?: GPUAddressMode;
  magFilter?: GPUFilterMode;
  minFilter?: GPUFilterMode;
}

/**
 * Empty 2D GPU texture with views for `texture_2d` sampling and `texture_storage_2d` writes.
 */
export class Texture2D {
  readonly width: number;
  readonly height: number;
  readonly format: GPUTextureFormat;
  readonly view: GPUTextureView;
  readonly storageView: GPUTextureView;
  readonly sampler: GPUSampler;

  private readonly _gpu: GPUTexture;

  private constructor(
    gpu: GPUTexture,
    view: GPUTextureView,
    storageView: GPUTextureView,
    sampler: GPUSampler,
    width: number,
    height: number,
    format: GPUTextureFormat,
  ) {
    this._gpu = gpu;
    this.view = view;
    this.storageView = storageView;
    this.sampler = sampler;
    this.width = width;
    this.height = height;
    this.format = format;
  }

  static create(
    device: Device,
    size: number | readonly [number, number],
    options: Texture2DOptions = {},
  ): Texture2D {
    const label = options.label ?? "Texture2D";
    const format = options.format ?? "rgba32float";
    const [width, height] = typeof size === "number" ? ([size, size] as const) : size;

    if (width <= 0 || height <= 0) {
      throw new Error("Texture2D size must have positive width and height.");
    }

    const usage =
      options.usage ?? GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.STORAGE_BINDING;

    const gpu = device.gpu.createTexture({
      label,
      dimension: "2d",
      size: [width, height, 1],
      format,
      usage,
    });

    const view = gpu.createView({ label: `${label}View`, dimension: "2d" });
    const storageView = view;
    const sampler = device.gpu.createSampler({
      label: `${label}Sampler`,
      addressModeU: options.addressModeU ?? "mirror-repeat",
      addressModeV: options.addressModeV ?? "mirror-repeat",
      magFilter: options.magFilter ?? "nearest",
      minFilter: options.minFilter ?? "nearest",
    });

    return new Texture2D(gpu, view, storageView, sampler, width, height, format);
  }

  /** Escape hatch for advanced consumers. */
  get gpu(): GPUTexture {
    return this._gpu;
  }

  destroy(): void {
    this._gpu.destroy();
  }
}
