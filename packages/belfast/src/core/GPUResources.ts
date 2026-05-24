import type { Device } from "./Device";

export function createShaderModule(
  device: Device,
  code: string,
  label?: string,
): GPUShaderModule {
  return device.device.createShaderModule({ code, label });
}

export function createRenderPipeline(
  device: Device,
  descriptor: GPURenderPipelineDescriptor,
): GPURenderPipeline {
  return device.device.createRenderPipeline(descriptor);
}

export function createBuffer(
  device: Device,
  size: number,
  usage: GPUBufferUsageFlags,
  label?: string,
): GPUBuffer {
  return device.device.createBuffer({ size, usage, label });
}

export function writeBuffer(
  device: Device,
  buffer: GPUBuffer,
  data: ArrayBuffer | ArrayBufferView,
  bufferOffset = 0,
): void {
  const source =
    data instanceof ArrayBuffer
      ? data
      : data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength);

  device.device.queue.writeBuffer(buffer, bufferOffset, source);
}
