import type { Device } from "./Device";
import { writeBuffer } from "./GPUResources";

export const BufferUsage = {
  vertex: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
  storage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
  uniform: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
  vertexStorage: GPUBufferUsage.VERTEX | GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
} as const;

export class Buffer {
  readonly gpu: GPUBuffer;
  readonly size: number;
  readonly usage: GPUBufferUsageFlags;
  readonly label?: string;

  private constructor(gpu: GPUBuffer, size: number, usage: GPUBufferUsageFlags, label?: string) {
    this.gpu = gpu;
    this.size = size;
    this.usage = usage;
    this.label = label;
  }

  /** Rounds byte length up to WGSL uniform struct alignment (16 bytes). */
  static uniformSize(byteLength: number): number {
    return Math.ceil(byteLength / 16) * 16;
  }

  static create(device: Device, size: number, usage: GPUBufferUsageFlags, label?: string): Buffer {
    const gpu = device.device.createBuffer({ size, usage, label });
    return new Buffer(gpu, size, usage, label);
  }

  static fromData(
    device: Device,
    data: ArrayBuffer | ArrayBufferView,
    usage: GPUBufferUsageFlags,
    label?: string,
  ): Buffer {
    const byteLength = data.byteLength;
    const buffer = Buffer.create(device, byteLength, usage, label);
    buffer.write(device, data);
    return buffer;
  }

  write(device: Device, data: ArrayBuffer | ArrayBufferView, byteOffset = 0): void {
    writeBuffer(device, this.gpu, data, byteOffset);
  }

  destroy(): void {
    this.gpu.destroy();
  }
}
