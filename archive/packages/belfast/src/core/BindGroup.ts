import type { Device } from "./Device";
import { Buffer } from "./Buffer";

export interface BindGroupResource {
  binding: number;
  resource: GPUBindingResource | Buffer;
}

function toBindingResource(resource: GPUBindingResource | Buffer): GPUBindingResource {
  if (resource instanceof Buffer) {
    return { buffer: resource.gpu };
  }
  return resource;
}

export class BindGroup {
  readonly gpu: GPUBindGroup;

  private constructor(gpu: GPUBindGroup) {
    this.gpu = gpu;
  }

  static create(
    device: Device,
    layout: GPUBindGroupLayout,
    buffer: Buffer,
    binding?: number,
    label?: string,
  ): BindGroup;
  static create(
    device: Device,
    layout: GPUBindGroupLayout,
    resources: BindGroupResource[],
    label?: string,
  ): BindGroup;
  static create(
    device: Device,
    layout: GPUBindGroupLayout,
    bufferOrResources: Buffer | BindGroupResource[],
    bindingOrLabel: number | string = 0,
    label?: string,
  ): BindGroup {
    if (bufferOrResources instanceof Buffer) {
      const binding = typeof bindingOrLabel === "number" ? bindingOrLabel : 0;
      const resolvedLabel = typeof bindingOrLabel === "string" ? bindingOrLabel : label;
      return BindGroup.createFromEntries(
        device,
        layout,
        [{ binding, resource: bufferOrResources }],
        resolvedLabel,
      );
    }

    const resolvedLabel = typeof bindingOrLabel === "string" ? bindingOrLabel : label;
    return BindGroup.createFromEntries(device, layout, bufferOrResources, resolvedLabel);
  }

  static createFromEntries(
    device: Device,
    layout: GPUBindGroupLayout,
    resources: BindGroupResource[],
    label?: string,
  ): BindGroup {
    const gpu = device.gpu.createBindGroup({
      label,
      layout,
      entries: resources.map(({ binding, resource }) => ({
        binding,
        resource: toBindingResource(resource),
      })),
    });
    return new BindGroup(gpu);
  }

  bind(passEncoder: GPURenderPassEncoder, groupIndex = 0): void {
    passEncoder.setBindGroup(groupIndex, this.gpu);
  }
}
