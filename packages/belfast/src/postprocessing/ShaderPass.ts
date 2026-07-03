import type { Device } from "../core/Device";
import { UniformBlock, type UniformBlockSchema } from "../core/UniformBlock";
import { Buffer, BufferUsage } from "../core/Buffer";
import { BindGroup } from "../core/BindGroup";
import { Draw } from "../helper/Draw";

const FULLSCREEN_VERTEX_SHADER = `
struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) VertexIndex: u32) -> VertexOutput {
  var pos = array<vec2<f32>, 3>(
    vec2<f32>(-1.0, -1.0),
    vec2<f32>( 3.0, -1.0),
    vec2<f32>(-1.0,  3.0)
  );
  var output: VertexOutput;
  output.position = vec4<f32>(pos[VertexIndex], 0.0, 1.0);
  output.uv = pos[VertexIndex] * 0.5 + 0.5;
  output.uv.y = 1.0 - output.uv.y;
  return output;
}
`;

export interface ShaderPassOptions {
  label?: string;
  uniforms?: UniformBlockSchema;
}

export class ShaderPass {
  readonly device: Device;
  readonly name: string;
  readonly uniforms?: UniformBlock;
  readonly uniformBuffer?: Buffer;

  protected draw: Draw;
  protected bindGroupLayout: GPUBindGroupLayout;
  protected currentInputView?: GPUTextureView;
  protected currentBindGroup?: BindGroup;
  protected sampler: GPUSampler;

  constructor(device: Device, fragmentShaderCode: string, options: ShaderPassOptions = {}) {
    this.device = device;
    const label = options.label ?? "ShaderPass";
    this.name = label;

    this.sampler = device.gpu.createSampler({
      label: `${label}Sampler`,
      addressModeU: "clamp-to-edge",
      addressModeV: "clamp-to-edge",
      magFilter: "linear",
      minFilter: "linear",
    });

    if (options.uniforms) {
      this.uniforms = UniformBlock.create(options.uniforms, `${label}Uniforms`);
      this.uniformBuffer = Buffer.create(
        device,
        Buffer.uniformSize(this.uniforms.byteSize),
        BufferUsage.uniform,
        `${label}UniformBuffer`,
      );
    }

    const shaderCode = `${FULLSCREEN_VERTEX_SHADER}\n${fragmentShaderCode}`;

    this.draw = new Draw(device, shaderCode, {
      label,
      layout: "auto",
      primitive: { topology: "triangle-list" },
      vertexBuffers: [],
    });

    this.bindGroupLayout = this.draw.getBindGroupLayout(0);
  }

  setUniform(name: string, value: number | ArrayLike<number>) {
    if (this.uniforms) {
      this.uniforms.set(name, value);
    }
  }

  render(pass: GPURenderPassEncoder, inputView: GPUTextureView) {
    if (this.uniforms && this.uniformBuffer) {
      this.uniforms.writeToBuffer(this.uniformBuffer, this.device);
    }

    if (this.currentInputView !== inputView || !this.currentBindGroup) {
      this.currentInputView = inputView;
      const entries: { binding: number; resource: GPUBindingResource }[] = [
        { binding: 0, resource: this.sampler },
        { binding: 1, resource: inputView },
      ];
      if (this.uniformBuffer) {
        entries.push({ binding: 2, resource: { buffer: this.uniformBuffer.gpu } });
      }

      this.currentBindGroup = BindGroup.create(
        this.device,
        this.bindGroupLayout,
        entries,
        "ShaderPassBindGroup",
      );
    }

    this.draw.draw(pass, 3, this.currentBindGroup);
  }

  destroy() {
    this.uniformBuffer?.destroy();
  }
}
