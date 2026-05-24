import type { Device } from "./Device";

export interface TextureOptions {
  label?: string;
  format?: GPUTextureFormat;
  /** Flip image vertically on upload (default true; browser images are top-left origin). */
  flipY?: boolean;
  addressModeU?: GPUAddressMode;
  addressModeV?: GPUAddressMode;
  magFilter?: GPUFilterMode;
  minFilter?: GPUFilterMode;
}

/**
 * 2D image uploaded to a `GPUTexture` with a default sampler for shader sampling.
 */
export class Texture {
  readonly width: number;
  readonly height: number;
  readonly view: GPUTextureView;
  readonly sampler: GPUSampler;

  private readonly gpu: GPUTexture;

  private constructor(
    gpu: GPUTexture,
    view: GPUTextureView,
    sampler: GPUSampler,
    width: number,
    height: number,
  ) {
    this.gpu = gpu;
    this.view = view;
    this.sampler = sampler;
    this.width = width;
    this.height = height;
  }

  static async load(device: Device, url: string, options: TextureOptions = {}): Promise<Texture> {
    const response = await fetch(url);
    if (!response.ok) {
      throw new Error(
        `Failed to load texture from ${url}: ${response.status} ${response.statusText}`,
      );
    }
    const blob = await response.blob();
    const bitmap = await createImageBitmap(blob);
    try {
      return Texture.fromBitmap(device, bitmap, options);
    } finally {
      bitmap.close();
    }
  }

  static fromBitmap(device: Device, bitmap: ImageBitmap, options: TextureOptions = {}): Texture {
    const label = options.label ?? "Texture";
    const format = options.format ?? "rgba8unorm";
    const width = bitmap.width;
    const height = bitmap.height;

    if (width <= 0 || height <= 0) {
      throw new Error("Texture source must have positive width and height.");
    }

    const gpu = device.device.createTexture({
      label,
      size: [width, height, 1],
      format,
      // RENDER_ATTACHMENT required by copyExternalImageToTexture (browser color-space fast path).
      usage:
        GPUTextureUsage.TEXTURE_BINDING |
        GPUTextureUsage.COPY_DST |
        GPUTextureUsage.RENDER_ATTACHMENT,
    });

    const flipY = options.flipY ?? true;
    device.device.queue.copyExternalImageToTexture({ source: bitmap, flipY }, { texture: gpu }, [
      width,
      height,
    ]);

    const view = gpu.createView({ label: `${label}View` });
    const sampler = device.device.createSampler({
      label: `${label}Sampler`,
      addressModeU: options.addressModeU ?? "clamp-to-edge",
      addressModeV: options.addressModeV ?? "clamp-to-edge",
      magFilter: options.magFilter ?? "linear",
      minFilter: options.minFilter ?? "linear",
    });

    return new Texture(gpu, view, sampler, width, height);
  }

  destroy(): void {
    this.gpu.destroy();
  }
}
