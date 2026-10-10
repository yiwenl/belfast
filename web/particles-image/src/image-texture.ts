import { BindGroup, Texture, type Device, type Draw } from "@belfast/wasm";

export interface ImageTextureBackend {
  white(): Promise<ImageBitmap>;
  decode(blob: Blob): Promise<ImageBitmap>;
  upload(bitmap: ImageBitmap): Texture;
  bind(texture: Texture): BindGroup;
}

export function createImageTextureBackend(device: Device, draw: Draw): ImageTextureBackend {
  return {
    white: () =>
      createImageBitmap(new ImageData(new Uint8ClampedArray([255, 255, 255, 255]), 1, 1)),
    decode: (blob) => createImageBitmap(blob),
    upload: (bitmap) =>
      Texture.fromImageBitmap(device, bitmap, {
        label: "ParticleImage",
        // Match the existing particle shader's output space: a browser unorm canvas
        // expects encoded RGB, while an sRGB render target performs output encoding.
        format: device.format().endsWith("Srgb") ? "rgba8unorm-srgb" : "rgba8unorm",
        flipY: true,
      }),
    bind: (texture) =>
      BindGroup.fromTexture(device, draw, texture, {
        groupIndex: 2,
        label: "ParticleImageBindGroup",
      }),
  };
}

export class ParticleImageTexture {
  private active?: { texture: Texture; bindGroup: BindGroup };
  private disposed = false;

  private constructor(private readonly backend: ImageTextureBackend) {}

  static async create(
    device: Device,
    draw: Draw,
    backend: ImageTextureBackend = createImageTextureBackend(device, draw),
  ): Promise<ParticleImageTexture> {
    const images = new ParticleImageTexture(backend);
    images.apply(await backend.white());
    return images;
  }

  get bindGroup(): BindGroup {
    if (!this.active) throw new Error("Image texture is disposed");
    return this.active.bindGroup;
  }

  async replace(blob: Blob): Promise<void> {
    if (this.disposed) throw new Error("Image texture is disposed");
    this.apply(await this.backend.decode(blob));
  }

  private apply(bitmap: ImageBitmap): void {
    let texture: Texture | undefined;
    try {
      if (this.disposed) throw new Error("Image texture is disposed");
      texture = this.backend.upload(bitmap);
      const bindGroup = this.backend.bind(texture);
      const previous = this.active;
      this.active = { texture, bindGroup };
      texture = undefined;
      previous?.bindGroup.free();
      previous?.texture.free();
    } finally {
      texture?.free();
      bitmap.close();
    }
  }

  dispose(): void {
    this.disposed = true;
    this.active?.bindGroup.free();
    this.active?.texture.free();
    this.active = undefined;
  }
}
