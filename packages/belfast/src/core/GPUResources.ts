import type { Device } from "./Device";

export function createShaderModule(device: Device, code: string, label?: string): GPUShaderModule {
  return device.gpu.createShaderModule({ code, label });
}

export function createRenderPipeline(
  device: Device,
  descriptor: GPURenderPipelineDescriptor,
): GPURenderPipeline {
  return device.gpu.createRenderPipeline(descriptor);
}

export function createComputePipeline(
  device: Device,
  descriptor: GPUComputePipelineDescriptor,
): GPUComputePipeline {
  return device.gpu.createComputePipeline(descriptor);
}

export function createBuffer(
  device: Device,
  size: number,
  usage: GPUBufferUsageFlags,
  label?: string,
): GPUBuffer {
  return device.gpu.createBuffer({ size, usage, label });
}

export function writeBuffer(
  device: Device,
  buffer: GPUBuffer,
  data: ArrayBuffer | ArrayBufferView,
  bufferOffset = 0,
): void {
  if (data instanceof ArrayBuffer) {
    device.gpu.queue.writeBuffer(buffer, bufferOffset, data);
    return;
  }

  device.gpu.queue.writeBuffer(buffer, bufferOffset, data.buffer, data.byteOffset, data.byteLength);
}
