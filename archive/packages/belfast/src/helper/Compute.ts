import type { BindGroup } from "../core/BindGroup";
import type { Device } from "../core/Device";
import { createComputePipeline, createShaderModule } from "../core/GPUResources";

export interface ComputeOptions {
  label?: string;
  /** Defaults to `"auto"`. Use a shared layout to reuse bind groups across pipelines. */
  layout?: GPUPipelineLayout | "auto";
  /** Compute entry point. Defaults to `"cs_main"`. */
  entryPoint?: string;
}

/** Number of workgroups to dispatch: `x`, `[x, y]`, or `[x, y, z]`. */
export type WorkgroupCount = number | readonly [number, number] | readonly [number, number, number];

/**
 * Compute pipeline wrapper mirroring {@link Draw}.
 * Encapsulates pipeline creation and reduces a compute dispatch to one call.
 */
export class Compute {
  private pipeline: GPUComputePipeline;

  constructor(device: Device, shaderCode: string, optionsOrLabel: ComputeOptions | string = {}) {
    const options = typeof optionsOrLabel === "string" ? { label: optionsOrLabel } : optionsOrLabel;
    const { label = "Compute", layout = "auto", entryPoint = "cs_main" } = options;

    const module = createShaderModule(device, shaderCode, `${label}Shader`);

    this.pipeline = createComputePipeline(device, {
      label: `${label}Pipeline`,
      layout,
      compute: { module, entryPoint },
    });
  }

  getBindGroupLayout(index = 0): GPUBindGroupLayout {
    return this.pipeline.getBindGroupLayout(index);
  }

  /** Records pipeline + bind groups + dispatch into an existing compute pass. */
  dispatch(
    passEncoder: GPUComputePassEncoder,
    bindGroup?: BindGroup | readonly BindGroup[],
    workgroups: WorkgroupCount = 1,
  ): void {
    passEncoder.setPipeline(this.pipeline);
    if (bindGroup) {
      const groups = Array.isArray(bindGroup) ? bindGroup : [bindGroup];
      for (let i = 0; i < groups.length; i++) {
        passEncoder.setBindGroup(i, groups[i].gpu);
      }
    }
    if (typeof workgroups === "number") {
      passEncoder.dispatchWorkgroups(workgroups);
    } else {
      passEncoder.dispatchWorkgroups(workgroups[0], workgroups[1] ?? 1, workgroups[2] ?? 1);
    }
  }

  /** Convenience: begin a compute pass, dispatch, and end it in a single call. */
  run(
    encoder: GPUCommandEncoder,
    bindGroup?: BindGroup | readonly BindGroup[],
    workgroups: WorkgroupCount = 1,
    label?: string,
  ): void {
    const pass = encoder.beginComputePass(label ? { label } : undefined);
    this.dispatch(pass, bindGroup, workgroups);
    pass.end();
  }
}
