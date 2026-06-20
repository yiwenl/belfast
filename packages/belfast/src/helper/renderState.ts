import type { DrawOptions } from "./Draw";
import type { DepthDrawOptions } from "./DepthDraw";

export interface TrianglePrimitiveStateOptions {
  topology?: GPUPrimitiveTopology;
  cullMode?: GPUCullMode;
  frontFace?: GPUFrontFace;
  stripIndexFormat?: GPUIndexFormat;
  unclippedDepth?: boolean;
}

export interface OpaqueTrianglesOptions extends TrianglePrimitiveStateOptions {
  colorFormat: GPUTextureFormat;
  depthFormat?: GPUTextureFormat;
  depthCompare?: GPUCompareFunction;
  depthWriteEnabled?: boolean;
}

export interface DepthOnlyTrianglesOptions extends TrianglePrimitiveStateOptions {
  depthFormat?: GPUTextureFormat;
  depthCompare?: GPUCompareFunction;
  depthWriteEnabled?: boolean;
}

export type OpaqueTrianglesState = Required<
  Pick<DrawOptions, "primitive" | "depthStencil" | "targets">
>;
export type DepthOnlyTrianglesState = Pick<
  DepthDrawOptions,
  "primitive" | "depthFormat" | "depthCompare" | "depthWriteEnabled"
>;

function trianglePrimitive(options: TrianglePrimitiveStateOptions = {}): GPUPrimitiveState {
  const {
    topology = "triangle-list",
    cullMode = "back",
    frontFace,
    stripIndexFormat,
    unclippedDepth,
  } = options;

  return {
    topology,
    cullMode,
    ...(frontFace ? { frontFace } : {}),
    ...(stripIndexFormat ? { stripIndexFormat } : {}),
    ...(unclippedDepth !== undefined ? { unclippedDepth } : {}),
  };
}

export function opaqueTriangles(options: OpaqueTrianglesOptions): OpaqueTrianglesState {
  const {
    colorFormat,
    depthFormat = "depth24plus",
    depthCompare = "less",
    depthWriteEnabled = true,
    ...primitiveOptions
  } = options;

  return {
    primitive: trianglePrimitive(primitiveOptions),
    depthStencil: {
      format: depthFormat,
      depthWriteEnabled,
      depthCompare,
    },
    targets: [{ format: colorFormat }],
  };
}

export function depthOnlyTriangles(
  options: DepthOnlyTrianglesOptions = {},
): DepthOnlyTrianglesState {
  const {
    depthFormat = "depth32float",
    depthCompare = "less",
    depthWriteEnabled = true,
    ...primitiveOptions
  } = options;

  return {
    primitive: trianglePrimitive(primitiveOptions),
    depthFormat,
    depthWriteEnabled,
    depthCompare,
  };
}
