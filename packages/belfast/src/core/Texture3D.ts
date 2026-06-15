import type { Device } from "./Device";

export interface Texture3DOptions {
  label?: string;
  format?: GPUTextureFormat;
  /** Override default `TEXTURE_BINDING | STORAGE_BINDING` when needed. */
  usage?: GPUTextureUsageFlags;
  addressModeU?: GPUAddressMode;
  addressModeV?: GPUAddressMode;
  addressModeW?: GPUAddressMode;
  magFilter?: GPUFilterMode;
  minFilter?: GPUFilterMode;
}

/**
 * Empty 3D GPU volume with views for `texture_3d` sampling and `texture_storage_3d` writes.
 */
export class Texture3D {
  readonly width: number;
  readonly height: number;
  readonly depth: number;
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
    depth: number,
    format: GPUTextureFormat,
  ) {
    this._gpu = gpu;
    this.view = view;
    this.storageView = storageView;
    this.sampler = sampler;
    this.width = width;
    this.height = height;
    this.depth = depth;
    this.format = format;
  }

  static create(
    device: Device,
    size: number | readonly [number, number, number],
    options: Texture3DOptions = {},
  ): Texture3D {
    const label = options.label ?? "Texture3D";
    const format = options.format ?? "rgba32float";
    const [width, height, depth] = typeof size === "number" ? ([size, size, size] as const) : size;

    if (width <= 0 || height <= 0 || depth <= 0) {
      throw new Error("Texture3D size must have positive width, height, and depth.");
    }

    const usage =
      options.usage ?? GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.STORAGE_BINDING;

    const gpu = device.gpu.createTexture({
      label,
      dimension: "3d",
      size: [width, height, depth],
      format,
      usage,
    });

    const view = gpu.createView({ label: `${label}View`, dimension: "3d" });
    const storageView = view;
    const sampler = device.gpu.createSampler({
      label: `${label}Sampler`,
      addressModeU: options.addressModeU ?? "mirror-repeat",
      addressModeV: options.addressModeV ?? "mirror-repeat",
      addressModeW: options.addressModeW ?? "mirror-repeat",
      magFilter: options.magFilter ?? "nearest",
      minFilter: options.minFilter ?? "nearest",
    });

    return new Texture3D(gpu, view, storageView, sampler, width, height, depth, format);
  }

  /** Escape hatch for advanced consumers. */
  get gpu(): GPUTexture {
    return this._gpu;
  }

  destroy(): void {
    this._gpu.destroy();
  }
}
