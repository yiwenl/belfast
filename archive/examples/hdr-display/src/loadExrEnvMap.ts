import type { Device } from "belfast";
import { readExr, readHdr } from "hdrify";

function floatToHalf(value: number): number {
  const f32 = new Float32Array(1);
  const u32 = new Uint32Array(f32.buffer);
  f32[0] = value;
  const bits = u32[0]!;
  const sign = (bits >> 16) & 0x8000;
  let exponent = ((bits >> 23) & 0xff) - 127 + 15;
  let mantissa = bits & 0x7fffff;

  if (exponent <= 0) {
    if (exponent < -10) {
      return sign;
    }
    mantissa |= 0x800000;
    const shift = 14 - exponent;
    mantissa += 1 << (shift - 1);
    return sign | (mantissa >> shift);
  }

  // Biased half exponent 31 is Inf/NaN; 30 is the largest finite exponent.
  if (exponent >= 31) {
    return sign | 0x7bff;
  }

  return sign | (exponent << 10) | (mantissa >> 13);
}

function float32RgbaToFloat16(src: Float32Array): Uint16Array {
  const out = new Uint16Array(src.length);
  for (let i = 0; i < src.length; i++) {
    out[i] = floatToHalf(src[i]!);
  }
  return out;
}

export interface EnvMapGpu {
  texture: GPUTexture;
  view: GPUTextureView;
  sampler: GPUSampler;
  width: number;
  height: number;
}

function decodeEnvMap(
  bytes: Uint8Array,
  url: string,
): { width: number; height: number; data: Float32Array } {
  const ext = url.split(/[?#]/)[0]?.split(".").pop()?.toLowerCase();
  if (ext === "hdr") {
    return readHdr(bytes);
  }
  if (ext === "exr") {
    return readExr(bytes);
  }
  throw new Error(`Unsupported environment map format "${ext ?? "unknown"}". Use .hdr or .exr.`);
}

function uploadRgba16FloatTexture(
  device: Device,
  width: number,
  height: number,
  rgba: Float32Array,
  label: string,
): GPUTexture {
  const pixels = float32RgbaToFloat16(rgba);
  const pixelBytes = new Uint8Array(pixels.buffer, pixels.byteOffset, pixels.byteLength);

  const texture = device.gpu.createTexture({
    label,
    size: [width, height, 1],
    format: "rgba16float",
    usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST,
  });

  device.gpu.queue.writeTexture(
    { texture },
    pixelBytes as BufferSource,
    { bytesPerRow: width * 8, rowsPerImage: height },
    [width, height, 1],
  );

  return texture;
}

/** Fetch an OpenEXR or Radiance HDR file, decode to linear RGBA, and upload as `rgba16float`. */
export async function loadExrEnvMap(device: Device, url: string): Promise<EnvMapGpu> {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`Failed to load environment map: ${response.status} ${response.statusText}`);
  }

  const { width, height, data } = decodeEnvMap(new Uint8Array(await response.arrayBuffer()), url);
  const texture = uploadRgba16FloatTexture(device, width, height, data, "env-map");

  const sampler = device.gpu.createSampler({
    label: "env-map-sampler",
    magFilter: "linear",
    minFilter: "linear",
    addressModeU: "repeat",
    addressModeV: "clamp-to-edge",
  });

  return {
    texture,
    view: texture.createView(),
    sampler,
    width,
    height,
  };
}
