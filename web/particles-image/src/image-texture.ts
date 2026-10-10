import { BindGroup, Texture, type Device, type Draw } from "@belfast/wasm";

export interface ImageTextureBackend {
  white(): Promise<ImageBitmap>;
  decode(blob: Blob): Promise<ImageBitmap>;
  upload(bitmap: ImageBitmap): Texture;
  bind(texture: Texture, slot: number): BindGroup;
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
    bind: (texture, slot) =>
      BindGroup.fromTexture(device, draw, texture, {
        groupIndex: 2 + slot,
        label: "ParticleImageBindGroup",
      }),
  };
}

export const COLOR_TRANSITION_SECONDS = 2;

type ImageSlot = { texture: Texture; bindGroup: BindGroup };

export class ParticleImageTexture {
  private readonly slots: ImageSlot[] = [];
  private disposed = false;
  private replacing = false;
  private progress = 1;
  private target = 0;

  private constructor(private readonly backend: ImageTextureBackend) {}

  static async create(
    device: Device,
    draw: Draw,
    backend: ImageTextureBackend = createImageTextureBackend(device, draw),
  ): Promise<ParticleImageTexture> {
    const images = new ParticleImageTexture(backend);
    try {
      for (let slot = 0; slot < 2; slot++) {
        images.slots.push(images.createSlot(await backend.white(), slot));
      }
      return images;
    } catch (error) {
      images.dispose();
      throw error;
    }
  }

  get bindGroups(): [BindGroup, BindGroup] {
    if (this.slots.length !== 2) throw new Error("Image textures are disposed");
    return [this.slots[0].bindGroup, this.slots[1].bindGroup];
  }

  get offset(): number {
    return this.progress;
  }
  get targetIndex(): number {
    return this.target;
  }
  get transitioning(): boolean {
    return this.progress < 1;
  }

  advance(deltaSeconds: number): void {
    if (this.disposed || !Number.isFinite(deltaSeconds) || deltaSeconds <= 0) return;
    this.progress = Math.min(1, this.progress + deltaSeconds / COLOR_TRANSITION_SECONDS);
  }

  async replace(blob: Blob): Promise<void> {
    if (this.disposed) throw new Error("Image textures are disposed");
    if (this.replacing || this.transitioning) throw new Error("Image transition is in progress");
    this.replacing = true;
    try {
      const next = 1 - this.target;
      const candidate = this.createSlot(await this.backend.decode(blob), next);
      const previous = this.slots[next];
      this.slots[next] = candidate;
      this.target = next;
      this.progress = 0;
      // Keep the source slot alive throughout the blend; only replace the inactive slot.
      previous.bindGroup.free();
      previous.texture.free();
    } finally {
      this.replacing = false;
    }
  }

  private createSlot(bitmap: ImageBitmap, slot: number): ImageSlot {
    let texture: Texture | undefined;
    try {
      if (this.disposed) throw new Error("Image textures are disposed");
      texture = this.backend.upload(bitmap);
      const bindGroup = this.backend.bind(texture, slot);
      const result = { texture, bindGroup };
      texture = undefined;
      return result;
    } finally {
      texture?.free();
      bitmap.close();
    }
  }

  dispose(): void {
    this.disposed = true;
    for (const slot of this.slots) {
      slot.bindGroup.free();
      slot.texture.free();
    }
    this.slots.length = 0;
  }
}
