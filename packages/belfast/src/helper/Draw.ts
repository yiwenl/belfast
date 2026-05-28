import type { BindGroup } from "../core/BindGroup";
import type { Device } from "../core/Device";
import type { Mesh } from "../core/Mesh";
import { createRenderPipeline, createShaderModule } from "../core/GPUResources";

export interface DrawOptions {
  label?: string;
  /** Defaults to `"auto"`. Use a shared layout (e.g. from `createSceneUniformPipelineLayout`) to reuse bind groups across pipelines. */
  layout?: GPUPipelineLayout | "auto";
  primitive?: GPUPrimitiveState;
  depthStencil?: GPUDepthStencilState;
  targets?: GPUColorTargetState[];
  vertexBuffers?: (GPUVertexBufferLayout | null)[];
}

export class Draw {
  private pipeline: GPURenderPipeline;

  constructor(device: Device, shaderCode: string, optionsOrLabel: DrawOptions | string = {}) {
    const options = typeof optionsOrLabel === "string" ? { label: optionsOrLabel } : optionsOrLabel;
    const {
      label = "Draw",
      layout = "auto",
      primitive = { topology: "triangle-list" },
      depthStencil,
      targets = [{ format: device.format }],
      vertexBuffers = [],
    } = options;

    const module = createShaderModule(device, shaderCode, `${label}Shader`);

    this.pipeline = createRenderPipeline(device, {
      label: `${label}Pipeline`,
      layout,
      vertex: {
        module,
        entryPoint: "vs_main",
        buffers: vertexBuffers,
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

  getBindGroupLayout(index = 0): GPUBindGroupLayout {
    return this.pipeline.getBindGroupLayout(index);
  }

  draw(
    passEncoder: GPURenderPassEncoder,
    meshOrVertexCount: Mesh | number,
    bindGroup?: BindGroup | readonly BindGroup[],
    instanceCount = 1,
  ): void {
    passEncoder.setPipeline(this.pipeline);
    if (bindGroup) {
      const groups = Array.isArray(bindGroup) ? bindGroup : [bindGroup];
      for (let i = 0; i < groups.length; i++) {
        groups[i].bind(passEncoder, i);
      }
    }
    if (typeof meshOrVertexCount === "number") {
      passEncoder.draw(meshOrVertexCount, instanceCount);
    } else {
      meshOrVertexCount.bind(passEncoder);
      if (meshOrVertexCount.hasIndexBuffer()) {
        passEncoder.drawIndexed(meshOrVertexCount.getIndexCount(), instanceCount);
      } else {
        passEncoder.draw(meshOrVertexCount.vertexCount, instanceCount);
      }
    }
  }
}
