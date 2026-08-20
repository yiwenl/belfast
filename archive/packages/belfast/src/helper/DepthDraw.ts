import type { Device } from "../core/Device";
import type { BindGroup } from "../core/BindGroup";
import type { Mesh } from "../core/Mesh";
import { Draw, type DrawOptions } from "./Draw";

export interface DepthDrawOptions extends Omit<DrawOptions, "targets"> {
  depthFormat?: GPUTextureFormat;
  depthCompare?: GPUCompareFunction;
  depthWriteEnabled?: boolean;
}

export class DepthDraw {
  private drawInternal: Draw;

  constructor(device: Device, shaderCode: string, optionsOrLabel: DepthDrawOptions | string = {}) {
    const options = typeof optionsOrLabel === "string" ? { label: optionsOrLabel } : optionsOrLabel;
    const {
      label = "DepthDraw",
      depthFormat = "depth32float",
      depthCompare = "less",
      depthWriteEnabled = true,
      ...restOptions
    } = options;

    const depthStencil: GPUDepthStencilState = options.depthStencil ?? {
      format: depthFormat,
      depthWriteEnabled,
      depthCompare,
    };

    this.drawInternal = new Draw(device, shaderCode, {
      label,
      targets: [],
      depthStencil,
      ...restOptions,
    });
  }

  getBindGroupLayout(index = 0): GPUBindGroupLayout {
    return this.drawInternal.getBindGroupLayout(index);
  }

  draw(
    passEncoder: GPURenderPassEncoder,
    meshOrVertexCount: Mesh | number,
    bindGroup?: BindGroup | readonly BindGroup[],
    instanceCount = 1,
  ): void {
    this.drawInternal.draw(passEncoder, meshOrVertexCount, bindGroup, instanceCount);
  }
}
