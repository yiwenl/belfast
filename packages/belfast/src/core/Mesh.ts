import type { Buffer } from "./Buffer";

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

/**
 * Vertex layout and buffer bindings for draw calls.
 * Named "Mesh" for now; in many engines mesh implies geometry + material — material/shader stay on Draw.
 */
export class Mesh {
  readonly vertexCount: number;
  private readonly bindings: VertexBufferBinding[] = [];

  constructor(vertexCount: number) {
    if (vertexCount <= 0) {
      throw new Error("Mesh vertexCount must be greater than 0.");
    }
    this.vertexCount = vertexCount;
  }

  addVertexBuffer(binding: VertexBufferBinding): this {
    const slot = binding.slot ?? this.bindings.length;

    if (this.bindings.some((entry) => (entry.slot ?? this.bindings.indexOf(entry)) === slot)) {
      throw new Error(`Vertex buffer slot ${slot} is already in use.`);
    }

    this.bindings.push({ ...binding, slot });
    return this;
  }

  getVertexLayouts(): GPUVertexBufferLayout[] {
    const layouts: GPUVertexBufferLayout[] = [];

    for (const binding of this.bindings) {
      const slot = binding.slot ?? layouts.length;
      layouts[slot] = {
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
      const slot = binding.slot ?? this.bindings.indexOf(binding);
      passEncoder.setVertexBuffer(slot, binding.buffer.gpu);
    }
  }
}
