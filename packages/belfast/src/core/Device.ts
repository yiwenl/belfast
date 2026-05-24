export interface DeviceOptions {
  powerPreference?: GPUPowerPreference;
  alpha?: boolean;
}

export class Device {
  readonly canvas: HTMLCanvasElement;
  readonly context: GPUCanvasContext;
  readonly device: GPUDevice;
  readonly format: GPUTextureFormat;

  private constructor(
    canvas: HTMLCanvasElement,
    context: GPUCanvasContext,
    device: GPUDevice,
    format: GPUTextureFormat,
  ) {
    this.canvas = canvas;
    this.context = context;
    this.device = device;
    this.format = format;
  }

  static async isSupported(): Promise<boolean> {
    if (!navigator.gpu) {
      return false;
    }
    const adapter = await navigator.gpu.requestAdapter();
    return adapter !== null;
  }

  static async create(canvas: HTMLCanvasElement, options: DeviceOptions = {}): Promise<Device> {
    if (!navigator.gpu) {
      throw new Error("WebGPU is not supported in this browser.");
    }

    const adapter = await navigator.gpu.requestAdapter({
      powerPreference: options.powerPreference,
    });

    if (!adapter) {
      throw new Error("Failed to request WebGPU adapter.");
    }

    const device = await adapter.requestDevice();
    const context = canvas.getContext("webgpu");

    if (!context) {
      throw new Error("Failed to get WebGPU canvas context.");
    }

    const format = navigator.gpu.getPreferredCanvasFormat();

    context.configure({
      device,
      format,
      alphaMode: options.alpha === false ? "opaque" : "premultiplied",
    });

    return new Device(canvas, context, device, format);
  }

  resize(width?: number, height?: number): void {
    const w = width ?? this.canvas.clientWidth;
    const h = height ?? this.canvas.clientHeight;

    if (this.canvas.width !== w || this.canvas.height !== h) {
      this.canvas.width = Math.max(1, w);
      this.canvas.height = Math.max(1, h);
    }
  }

  getCurrentTexture(): GPUTexture {
    return this.context.getCurrentTexture();
  }

  destroy(): void {
    this.device.destroy();
  }
}
