import { BindGroup } from "../core/BindGroup";
import type { Device } from "../core/Device";
import { Draw } from "./Draw";

const COPY_SHADER = /* wgsl */ `
@group(0) @binding(0) var sourceTexture: texture_2d<f32>;
@group(0) @binding(1) var sourceSampler: sampler;

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertexIndex: u32) -> VertexOutput {
  var output: VertexOutput;
  var positions = array<vec2<f32>, 3>(
    vec2<f32>(-1.0, -1.0),
    vec2<f32>(3.0, -1.0),
    vec2<f32>(-1.0, 3.0),
  );
  let pos = positions[vertexIndex];
  output.position = vec4<f32>(pos, 0.0, 1.0);
  output.uv = vec2<f32>(pos.x * 0.5 + 0.5, -pos.y * 0.5 + 0.5);
  return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
  return textureSample(sourceTexture, sourceSampler, input.uv);
}
`;

export interface CopyHelperOptions {
  label?: string;
  targets?: GPUColorTargetState[];
}

export interface CopyDrawOptions {
  x?: number;
  y?: number;
  width?: number;
  height?: number;
}

export class CopyHelper {
  private readonly device: Device;
  private readonly drawPass: Draw;
  private readonly bindGroupLayout: GPUBindGroupLayout;
  private cachedTextureView?: GPUTextureView;
  private cachedSampler?: GPUSampler;
  private cachedBindGroup?: BindGroup;

  constructor(device: Device, options: CopyHelperOptions = {}) {
    this.device = device;
    const label = options.label ?? "CopyHelper";
    this.drawPass = new Draw(device, COPY_SHADER, {
      label,
      targets: options.targets ?? [{ format: device.format }],
      primitive: { topology: "triangle-list", cullMode: "none" },
    });
    this.bindGroupLayout = this.drawPass.getBindGroupLayout(0);
  }

  draw(
    passEncoder: GPURenderPassEncoder,
    textureView: GPUTextureView,
    sampler: GPUSampler,
    options: CopyDrawOptions = {},
  ): void {
    if (
      !this.cachedBindGroup ||
      this.cachedTextureView !== textureView ||
      this.cachedSampler !== sampler
    ) {
      this.cachedTextureView = textureView;
      this.cachedSampler = sampler;
      this.cachedBindGroup = BindGroup.create(
        this.device,
        this.bindGroupLayout,
        [
          { binding: 0, resource: textureView },
          { binding: 1, resource: sampler },
        ],
        "copy-helper-bind-group",
      );
    }
    if (
      options.x !== undefined ||
      options.y !== undefined ||
      options.width !== undefined ||
      options.height !== undefined
    ) {
      passEncoder.setViewport(
        options.x ?? 0,
        options.y ?? 0,
        options.width ?? this.device.canvas.width,
        options.height ?? this.device.canvas.height,
        0,
        1,
      );
      passEncoder.setScissorRect(
        Math.floor(options.x ?? 0),
        Math.floor(options.y ?? 0),
        Math.floor(options.width ?? this.device.canvas.width),
        Math.floor(options.height ?? this.device.canvas.height),
      );
    }
    this.drawPass.draw(passEncoder, 3, this.cachedBindGroup);
  }
}
