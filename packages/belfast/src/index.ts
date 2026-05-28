/// <reference types="@webgpu/types" />

export { OrbitalControl, type OrbitalControlOptions } from "./controls/OrbitalControl";
export { EaseNumber } from "./utils/EaseNumber";
export { Camera } from "./camera/Camera";
export { PerspectiveCamera } from "./camera/PerspectiveCamera";
export { OrthographicCamera } from "./camera/OrthographicCamera";
export type { ReadonlyVec3 as Vec3, vec3 as MutVec3 } from "gl-matrix";
export type Mat4 = Float32Array;
export { Device, type DeviceOptions } from "./core/Device";
export { BindGroup, type BindGroupResource } from "./core/BindGroup";
export { Buffer, BufferUsage } from "./core/Buffer";
export { UniformBlock, type UniformFieldType, type UniformBlockSchema } from "./core/UniformBlock";
export { Texture, type TextureOptions } from "./core/Texture";
export { RenderTarget, type RenderTargetOptions } from "./core/RenderTarget";
export { beginRenderPass, type RenderPassOptions } from "./core/RenderPass";
export { Draw, type DrawOptions } from "./helper/Draw";
export { AxisHelper, type AxisHelperOptions } from "./helper/AxisHelper";
export { BallHelper, type BallHelperOptions, type BallDrawParams } from "./helper/BallHelper";
export { CopyHelper, type CopyHelperOptions } from "./helper/CopyHelper";
export {
  Geom,
  type GeometryData,
  type PlaneOptions,
  type SphereOptions,
  type CubeOptions,
} from "./helper/Geom";
export {
  createSceneUniformBindGroupLayout,
  createSceneUniformPipelineLayout,
  createBallInstanceBindGroupLayout,
  createSceneBallPipelineLayout,
  createSceneTextureBindGroupLayout,
  createSceneTexturePipelineLayout,
} from "./helper/sceneLayout";
export {
  Mesh,
  type VertexAttributeDescriptor,
  type VertexBufferBinding,
  type MeshIndexFormat,
} from "./core/Mesh";
export {
  createPlaneTriangleList,
  createBillboardDiscTriangle,
  type PlaneAxis,
  type PlaneTriangleList,
  type BillboardDiscTriangle,
} from "./geom/plane";

import { Device } from "./core/Device";

export function showWebGPUUnavailableMessage(container: ParentNode = document.body): void {
  const message = document.createElement("div");
  message.style.cssText =
    "position:fixed;inset:0;display:flex;align-items:center;justify-content:center;padding:2rem;font:16px/1.5 system-ui,sans-serif;background:#111;color:#eee;text-align:center;";
  message.textContent =
    "WebGPU is not available in this browser. Try the latest Chrome, Edge, or Safari.";
  container.appendChild(message);
}

export async function assertWebGPUSupport(): Promise<void> {
  if (!(await Device.isSupported())) {
    showWebGPUUnavailableMessage();
    throw new Error("WebGPU is not supported.");
  }
}
