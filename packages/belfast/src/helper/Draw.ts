import type { Device } from "../core/Device";
import { createRenderPipeline, createShaderModule } from "../core/GPUResources";

export interface DrawOptions {
  label?: string;
  primitive?: GPUPrimitiveState;
  depthStencil?: GPUDepthStencilState;
  targets?: GPUColorTargetState[];
}

export class Draw {
  private pipeline: GPURenderPipeline;

  constructor(device: Device, shaderCode: string, optionsOrLabel: DrawOptions | string = {}) {
    const options = typeof optionsOrLabel === "string" ? { label: optionsOrLabel } : optionsOrLabel;
    const {
      label = "Draw",
      primitive = { topology: "triangle-list" },
      depthStencil,
      targets = [{ format: device.format }],
    } = options;

    const module = createShaderModule(device, shaderCode, `${label}Shader`);

    this.pipeline = createRenderPipeline(device, {
      label: `${label}Pipeline`,
      layout: "auto",
      vertex: {
        module,
        entryPoint: "vs_main",
      },
      fragment: {
        module,
        entryPoint: "fs_main",
        targets,
      },
      primitive,
      depthStencil,
    });
  }

  draw(passEncoder: GPURenderPassEncoder, vertexCount = 3, instanceCount = 1): void {
    passEncoder.setPipeline(this.pipeline);
    passEncoder.draw(vertexCount, instanceCount);
  }
}
