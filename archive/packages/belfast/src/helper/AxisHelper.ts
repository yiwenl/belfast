import type { BindGroup } from "../core/BindGroup";
import type { Device } from "../core/Device";
import { Buffer, BufferUsage } from "../core/Buffer";
import { Mesh } from "../core/Mesh";
import { Draw } from "./Draw";
import { createSceneUniformPipelineLayout } from "./sceneLayout";

const AXIS_SHADER = /* wgsl */ `
struct SceneUniforms {
  viewProj: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> scene: SceneUniforms;

struct VertexInput {
  @location(0) position: vec3<f32>,
  @location(1) color: vec3<f32>,
}

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) color: vec3<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
  var output: VertexOutput;
  output.position = scene.viewProj * vec4<f32>(input.position, 1.0);
  output.color = input.color;
  return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
  return vec4<f32>(input.color, 1.0);
}
`;

const DEFAULT_LENGTH = 1000;

export interface AxisHelperOptions {
  /** Half-extent of each axis line (default 1000, matching alfrid DrawAxis). */
  length?: number;
  label?: string;
  /**
   * Shared with other scene draws (e.g. triangle `Draw`) so one bind group works.
   * From `createSceneUniformPipelineLayout(device).pipelineLayout`.
   */
  pipelineLayout?: GPUPipelineLayout;
}

function buildAxisGeometry(length: number): {
  positions: Float32Array;
  colors: Float32Array;
} {
  const positions = new Float32Array([
    -length,
    0,
    0,
    length,
    0,
    0,
    0,
    -length,
    0,
    0,
    length,
    0,
    0,
    0,
    -length,
    0,
    0,
    length,
  ]);
  const colors = new Float32Array([1, 0, 0, 1, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 1, 0, 0, 1]);
  return { positions, colors };
}

/**
 * RGB axis lines along X (red), Y (green), and Z (blue).
 * Uses `line-list` topology; share a view-projection bind group with other scene draws.
 */
export class AxisHelper {
  readonly mesh: Mesh;
  private readonly lineDraw: Draw;
  private readonly positionBuffer: Buffer;
  private readonly colorBuffer: Buffer;

  constructor(device: Device, options: AxisHelperOptions = {}) {
    const length = options.length ?? DEFAULT_LENGTH;
    const label = options.label ?? "AxisHelper";
    const { vertex } = BufferUsage;
    const { positions, colors } = buildAxisGeometry(length);

    this.positionBuffer = Buffer.fromData(device, positions, vertex, `${label}-positions`);
    this.colorBuffer = Buffer.fromData(device, colors, vertex, `${label}-colors`);

    this.mesh = new Mesh(6)
      .addVertexBuffer({
        buffer: this.positionBuffer,
        arrayStride: 12,
        attributes: [{ shaderLocation: 0, format: "float32x3", offset: 0 }],
        slot: 0,
      })
      .addVertexBuffer({
        buffer: this.colorBuffer,
        arrayStride: 12,
        attributes: [{ shaderLocation: 1, format: "float32x3", offset: 0 }],
        slot: 1,
      });

    const pipelineLayout =
      options.pipelineLayout ??
      createSceneUniformPipelineLayout(device, `${label}Scene`).pipelineLayout;

    this.lineDraw = new Draw(device, AXIS_SHADER, {
      label,
      layout: pipelineLayout,
      primitive: { topology: "line-list" },
      vertexBuffers: this.mesh.getVertexLayouts(),
      depthStencil: {
        format: "depth24plus",
        depthWriteEnabled: true,
        depthCompare: "less",
      },
    });
  }

  /** Returns bind group layout index 0 (same `SceneUniforms` as other camera-lit helpers). */
  getBindGroupLayout(index = 0): GPUBindGroupLayout {
    return this.lineDraw.getBindGroupLayout(index);
  }

  draw(passEncoder: GPURenderPassEncoder, bindGroup: BindGroup): void {
    this.lineDraw.draw(passEncoder, this.mesh, bindGroup);
  }

  destroy(): void {
    this.positionBuffer.destroy();
    this.colorBuffer.destroy();
  }
}
