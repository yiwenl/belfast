export interface DeviceOptions {
  powerPreference?: GPUPowerPreference;
  alpha?: boolean;
  /**
   * Enable HDR-oriented defaults:
   * - canvas format: `rgba16float`
   * - canvas colorSpace: `srgb` (Rec.709 — matches typical .hdr/.exr env maps)
   * - toneMapping: `{ mode: "extended" }`
   */
  hdr?: boolean;
  /** Canvas color space for presentation. Defaults to "srgb". */
  colorSpace?: PredefinedColorSpace;
  /** Canvas tone mapping mode. Defaults to "standard". */
  toneMappingMode?: GPUCanvasToneMappingMode;
}

export class Device {
  readonly canvas: HTMLCanvasElement;
  readonly context: GPUCanvasContext;
  readonly device: GPUDevice;
  readonly format: GPUTextureFormat;
  readonly colorSpace: PredefinedColorSpace;
  readonly toneMappingMode: GPUCanvasToneMappingMode;
  readonly hdr: boolean;

  private constructor(
    canvas: HTMLCanvasElement,
    context: GPUCanvasContext,
    device: GPUDevice,
    format: GPUTextureFormat,
    colorSpace: PredefinedColorSpace,
    toneMappingMode: GPUCanvasToneMappingMode,
    hdr: boolean,
  ) {
    this.canvas = canvas;
    this.context = context;
    this.device = device;
    this.format = format;
    this.colorSpace = colorSpace;
    this.toneMappingMode = toneMappingMode;
    this.hdr = hdr;
  }

  /** Alias for the underlying `GPUDevice`, matching the `.gpu` convention on Buffer/BindGroup/Texture. */
  get gpu(): GPUDevice {
    return this.device;
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

    const hdr = options.hdr ?? false;
    const colorSpace = options.colorSpace ?? "srgb";
    const toneMappingMode = options.toneMappingMode ?? (hdr ? "extended" : "standard");
    // Extended HDR presentation needs a float swapchain; 8-bit formats clamp > 1.0.
    const format: GPUTextureFormat = hdr ? "rgba16float" : navigator.gpu.getPreferredCanvasFormat();

    context.configure({
      device,
      format,
      alphaMode: options.alpha === false ? "opaque" : "premultiplied",
      colorSpace,
      toneMapping: { mode: toneMappingMode },
    });

    return new Device(canvas, context, device, format, colorSpace, toneMappingMode, hdr);
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
