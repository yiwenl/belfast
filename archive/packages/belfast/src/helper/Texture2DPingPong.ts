import type { Device } from "../core/Device";
import { Texture2D, type Texture2DOptions } from "../core/Texture2D";

export interface Texture2DPingPongOptions extends Texture2DOptions {}

/**
 * Helper to manage two `Texture2D` instances for iterative compute read/write passes.
 */
export class Texture2DPingPong {
  private _read: Texture2D;
  private _write: Texture2D;

  private constructor(read: Texture2D, write: Texture2D) {
    this._read = read;
    this._write = write;
  }

  static create(
    device: Device,
    size: number | readonly [number, number],
    options: Texture2DPingPongOptions = {},
  ): Texture2DPingPong {
    const label = options.label ?? "Texture2DPingPong";
    const read = Texture2D.create(device, size, { ...options, label: `${label}-A` });
    const write = Texture2D.create(device, size, { ...options, label: `${label}-B` });
    return new Texture2DPingPong(read, write);
  }

  get read(): Texture2D {
    return this._read;
  }

  get write(): Texture2D {
    return this._write;
  }

  swap(): void {
    const temp = this._read;
    this._read = this._write;
    this._write = temp;
  }

  destroy(): void {
    this._read.destroy();
    this._write.destroy();
  }
}
