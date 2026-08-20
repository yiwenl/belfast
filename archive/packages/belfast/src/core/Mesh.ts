import { Buffer, BufferUsage } from "./Buffer";
import type { Device } from "./Device";

export interface VertexAttributeDescriptor {
  shaderLocation: number;
  format: GPUVertexFormat;
  offset: number;
}

export interface VertexBufferBinding {
  buffer: Buffer;
  arrayStride: number;
  attributes: VertexAttributeDescriptor[];
  slot?: number;
  stepMode?: GPUVertexStepMode;
}

export type MeshIndexFormat = "uint16" | "uint32";

interface ResolvedVertexBufferBinding extends VertexBufferBinding {
  slot: number;
}

/**
 * Vertex layout and buffer bindings for draw calls.
 * Named "Mesh" for now; in many engines mesh implies geometry + material — material/shader stay on Draw.
 */
export class Mesh {
  readonly vertexCount: number;
  private readonly bindings: ResolvedVertexBufferBinding[] = [];
  private indexBuffer?: Buffer;
  private indexCount = 0;
  private indexFormat: MeshIndexFormat = "uint16";

  constructor(vertexCount: number) {
    if (vertexCount <= 0) {
      throw new Error("Mesh vertexCount must be greater than 0.");
    }
    this.vertexCount = vertexCount;
  }

  addVertexBuffer(binding: VertexBufferBinding): this {
    const slot = binding.slot ?? this.nextFreeSlot();

    if (this.bindings.some((entry) => entry.slot === slot)) {
      throw new Error(`Vertex buffer slot ${slot} is already in use.`);
    }

    this.bindings.push({ ...binding, slot });
    return this;
  }

  getVertexLayouts(): (GPUVertexBufferLayout | null)[] {
    if (this.bindings.length === 0) {
      return [];
    }

    const maxSlot = Math.max(...this.bindings.map((binding) => binding.slot));
    const layouts: (GPUVertexBufferLayout | null)[] = Array.from(
      { length: maxSlot + 1 },
      () => null,
    );

    for (const binding of this.bindings) {
      layouts[binding.slot] = {
        arrayStride: binding.arrayStride,
        stepMode: binding.stepMode ?? "vertex",
        attributes: binding.attributes.map((attribute) => ({
          shaderLocation: attribute.shaderLocation,
          format: attribute.format,
          offset: attribute.offset,
        })),
      };
    }

    return layouts;
  }

  bind(passEncoder: GPURenderPassEncoder): void {
    for (const binding of this.bindings) {
      passEncoder.setVertexBuffer(binding.slot, binding.buffer.gpu);
    }
    if (this.indexBuffer) {
      passEncoder.setIndexBuffer(this.indexBuffer.gpu, this.indexFormat);
    }
  }

  setIndexBuffer(buffer: Buffer, count: number, format: MeshIndexFormat = "uint16"): this {
    if (count <= 0) {
      throw new Error("Mesh index count must be greater than 0.");
    }
    this.indexBuffer = buffer;
    this.indexCount = count;
    this.indexFormat = format;
    return this;
  }

  setIndexBufferFromData(
    device: Device,
    indices: Uint16Array | Uint32Array,
    label = "mesh-indices",
  ): Buffer {
    const format: MeshIndexFormat = indices instanceof Uint32Array ? "uint32" : "uint16";
    const buffer = Buffer.fromData(device, indices, BufferUsage.index, label);
    this.setIndexBuffer(buffer, indices.length, format);
    return buffer;
  }

  hasIndexBuffer(): boolean {
    return this.indexBuffer !== undefined;
  }

  getIndexCount(): number {
    return this.indexCount;
  }

  private nextFreeSlot(): number {
    const occupied = new Set(this.bindings.map((binding) => binding.slot));
    let slot = 0;
    while (occupied.has(slot)) {
      slot++;
    }
    return slot;
  }
}
