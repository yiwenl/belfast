import { Texture3D, type Texture3DOptions } from "../core/Texture3D";
import type { Device } from "../core/Device";

export type Texture3DPingPongOptions = Texture3DOptions;

/**
 * Two {@link Texture3D} volumes for compute read/write ping-pong.
 * Mirrors Alfrid `FboPingPong`: `read` is the last completed write, `write` is the current target.
 */
export class Texture3DPingPong {
  private textures: [Texture3D, Texture3D];
  readonly size: number;

  private constructor(textures: [Texture3D, Texture3D], size: number) {
    this.textures = textures;
    this.size = size;
  }

  static create(
    device: Device,
    size: number | readonly [number, number, number],
    options: Texture3DPingPongOptions = {},
  ): Texture3DPingPong {
    const label = options.label ?? "Texture3DPingPong";
    const [width, height, depth] = typeof size === "number" ? ([size, size, size] as const) : size;
    const maxDim = Math.max(width, height, depth);

    const textures: [Texture3D, Texture3D] = [
      Texture3D.create(device, [width, height, depth], { ...options, label: `${label}-write` }),
      Texture3D.create(device, [width, height, depth], { ...options, label: `${label}-read` }),
    ];

    return new Texture3DPingPong(textures, maxDim);
  }

  /** Last completed write — bind as `texture_3d` input. */
  get read(): Texture3D {
    return this.textures[1];
  }

  /** Current pass target — bind as `texture_storage_3d` output. */
  get write(): Texture3D {
    return this.textures[0];
  }

  /** Exchange read/write after a compute pass. */
  swap(): void {
    const [write, read] = this.textures;
    this.textures = [read, write];
  }

  destroy(): void {
    for (const texture of this.textures) {
      texture.destroy();
    }
  }
}
