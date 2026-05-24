import type { Device } from "../core/Device";
import { createRenderPipeline, createShaderModule } from "../core/GPUResources";

export class Draw {
  private pipeline: GPURenderPipeline;

  constructor(device: Device, shaderCode: string, label = "Draw") {
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
        targets: [{ format: device.format }],
      },
      primitive: {
        topology: "triangle-list",
      },
    });
  }

  draw(passEncoder: GPURenderPassEncoder, vertexCount = 3, instanceCount = 1): void {
    passEncoder.setPipeline(this.pipeline);
    passEncoder.draw(vertexCount, instanceCount);
  }
}
