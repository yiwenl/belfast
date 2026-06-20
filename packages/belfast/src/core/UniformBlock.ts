import type { Buffer } from "./Buffer";
import type { Device } from "./Device";

export type UniformFieldType = "f32" | "u32" | "vec2f" | "vec3f" | "vec4f" | "mat4x4f";
export type UniformBlockSchema = Record<string, UniformFieldType>;

interface UniformFieldMeta {
  type: UniformFieldType;
  floatOffset: number;
  valueFloatCount: number;
}

interface UniformTypeSpec {
  alignment: number;
  storageByteSize: number;
  valueFloatCount: number;
}

const TYPE_SPECS: Record<UniformFieldType, UniformTypeSpec> = {
  f32: { alignment: 4, storageByteSize: 4, valueFloatCount: 1 },
  u32: { alignment: 4, storageByteSize: 4, valueFloatCount: 1 },
  vec2f: { alignment: 8, storageByteSize: 8, valueFloatCount: 2 },
  vec3f: { alignment: 16, storageByteSize: 16, valueFloatCount: 3 },
  vec4f: { alignment: 16, storageByteSize: 16, valueFloatCount: 4 },
  mat4x4f: { alignment: 16, storageByteSize: 64, valueFloatCount: 16 },
};

const MAX_U32 = 0xffffffff;

function alignTo(value: number, alignment: number): number {
  const remainder = value % alignment;
  return remainder === 0 ? value : value + alignment - remainder;
}

function isArrayLikeNumber(value: unknown): value is ArrayLike<number> {
  return typeof value === "object" && value !== null && "length" in value;
}

export class UniformBlock {
  readonly floatCount: number;
  readonly byteSize: number;
  readonly label?: string;

  private readonly buffer: ArrayBuffer;
  private readonly dataInternal: Float32Array;
  private readonly uintDataInternal: Uint32Array;
  private readonly fields = new Map<string, UniformFieldMeta>();

  private constructor(schema: UniformBlockSchema, label?: string) {
    this.label = label;

    let byteOffset = 0;
    for (const [name, type] of Object.entries(schema)) {
      const spec = TYPE_SPECS[type];
      if (!spec) {
        throw new Error(`Unsupported uniform field type "${type}" for "${name}".`);
      }
      byteOffset = alignTo(byteOffset, spec.alignment);
      this.fields.set(name, {
        type,
        floatOffset: byteOffset / 4,
        valueFloatCount: spec.valueFloatCount,
      });
      byteOffset += spec.storageByteSize;
    }

    this.byteSize = byteOffset;
    this.floatCount = this.byteSize / 4;
    this.buffer = new ArrayBuffer(this.byteSize);
    this.dataInternal = new Float32Array(this.buffer);
    this.uintDataInternal = new Uint32Array(this.buffer);
  }

  static create(schema: UniformBlockSchema, label?: string): UniformBlock {
    return new UniformBlock(schema, label);
  }

  get data(): Readonly<Float32Array> {
    return this.dataInternal;
  }

  getOffset(name: string): number {
    const field = this.fields.get(name);
    if (!field) {
      throw new Error(`Unknown uniform field "${name}".`);
    }
    return field.floatOffset;
  }

  set(name: string, value: number | ArrayLike<number>): this {
    const field = this.fields.get(name);
    if (!field) {
      throw new Error(`Unknown uniform field "${name}".`);
    }

    if (field.type === "f32") {
      if (typeof value !== "number") {
        throw new Error(`Field "${name}" expects a number (f32).`);
      }
      this.dataInternal[field.floatOffset] = value;
      return this;
    }

    if (field.type === "u32") {
      if (typeof value !== "number") {
        throw new Error(`Field "${name}" expects a number (u32).`);
      }
      if (!Number.isFinite(value) || !Number.isInteger(value) || value < 0 || value > MAX_U32) {
        throw new Error(`Field "${name}" expects a u32 integer between 0 and ${MAX_U32}.`);
      }
      this.uintDataInternal[field.floatOffset] = value;
      return this;
    }

    if (typeof value === "number" || !isArrayLikeNumber(value)) {
      throw new Error(
        `Field "${name}" expects ${field.valueFloatCount} floats for type "${field.type}".`,
      );
    }
    if (value.length < field.valueFloatCount) {
      throw new Error(
        `Field "${name}" requires ${field.valueFloatCount} floats; got ${value.length}.`,
      );
    }

    if (value instanceof Float32Array) {
      this.dataInternal.set(value.subarray(0, field.valueFloatCount), field.floatOffset);
    } else {
      const len = field.valueFloatCount;
      const offset = field.floatOffset;
      for (let i = 0; i < len; i++) {
        this.dataInternal[offset + i] = value[i];
      }
    }
    if (field.type === "vec3f") {
      this.dataInternal[field.floatOffset + 3] = 0;
    }
    return this;
  }

  toFloat32Array(): Float32Array {
    return this.dataInternal;
  }

  writeToBuffer(buffer: Buffer, device: Device, byteOffset = 0): void {
    buffer.write(device, this.dataInternal, byteOffset);
  }
}
