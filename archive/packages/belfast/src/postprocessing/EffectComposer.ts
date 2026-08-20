import type { Device } from "../core/Device";
import { Texture2DPingPong } from "../helper/Texture2DPingPong";
import { beginRenderPass } from "../core/RenderPass";
import type { ShaderPass } from "./ShaderPass";

export interface EffectComposerOptions {
  format?: GPUTextureFormat;
}

export class EffectComposer {
  private device: Device;
  public passes: ShaderPass[] = [];
  private pingPong: Texture2DPingPong;
  private width: number;
  private height: number;

  constructor(device: Device, width: number, height: number, options: EffectComposerOptions = {}) {
    this.device = device;
    this.width = Math.max(1, Math.floor(width));
    this.height = Math.max(1, Math.floor(height));
    const format = options.format ?? device.format;

    this.pingPong = Texture2DPingPong.create(device, [this.width, this.height], {
      format,
      usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING,
      label: "EffectComposerPingPong",
    });
  }

  addPass(pass: ShaderPass) {
    this.passes.push(pass);
  }

  clearPasses() {
    this.passes = [];
  }

  setPasses(passes: ShaderPass[]) {
    this.passes = [...passes];
  }

  resize(width: number, height: number) {
    const w = Math.max(1, Math.floor(width));
    const h = Math.max(1, Math.floor(height));
    if (this.width === w && this.height === h) return;
    this.width = w;
    this.height = h;

    const format = this.pingPong.read.format;
    this.pingPong.destroy();
    this.pingPong = Texture2DPingPong.create(this.device, [w, h], {
      format,
      usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING,
      label: "EffectComposerPingPong",
    });
  }

  render(
    encoder: GPUCommandEncoder,
    inputTextureView: GPUTextureView,
    outputTextureView: GPUTextureView,
  ) {
    if (this.passes.length === 0) {
      return;
    }

    let currentInput = inputTextureView;

    for (let i = 0; i < this.passes.length; i++) {
      const isLast = i === this.passes.length - 1;
      const pass = this.passes[i];

      const renderTargetView = isLast ? outputTextureView : this.pingPong.write.view;

      const renderPass = beginRenderPass(encoder, renderTargetView, {
        clearColor: { r: 0, g: 0, b: 0, a: 0 },
      });

      pass.render(renderPass, currentInput);
      renderPass.end();

      if (!isLast) {
        currentInput = this.pingPong.write.view;
        this.pingPong.swap();
      }
    }
  }

  destroy() {
    this.pingPong.destroy();
    for (const pass of this.passes) {
      pass.destroy();
    }
  }
}
