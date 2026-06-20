/// <reference types="@webgpu/types" />

export { OrbitalControl, type OrbitalControlOptions } from "./controls/OrbitalControl";
export { EaseNumber } from "./utils/EaseNumber";
export { Ray } from "./math/Ray";
export { HitTestor, type HitTestorOptions, type HitDetail } from "./utils/HitTestor";
export { Camera } from "./camera/Camera";
export { PerspectiveCamera } from "./camera/PerspectiveCamera";
export { OrthographicCamera } from "./camera/OrthographicCamera";
export {
  fitOrthographicCameraToSphere,
  fitOrthographicCameraToBounds,
  type FitOrthographicCameraToSphereOptions,
  type FitOrthographicCameraToBoundsOptions,
} from "./camera/cameraFit";
export type { ReadonlyVec3 as Vec3, vec3 as MutVec3 } from "gl-matrix";
export type Mat4 = Float32Array;
export { Device, type DeviceOptions } from "./core/Device";
export { BindGroup, type BindGroupResource } from "./core/BindGroup";
export { Buffer, BufferUsage } from "./core/Buffer";
export { UniformBlock, type UniformFieldType, type UniformBlockSchema } from "./core/UniformBlock";
export { Texture, type TextureOptions } from "./core/Texture";
export { Texture2D, type Texture2DOptions } from "./core/Texture2D";
export { Texture3D, type Texture3DOptions } from "./core/Texture3D";
export { RenderTarget, type RenderTargetOptions } from "./core/RenderTarget";
export { beginRenderPass, type RenderPassOptions } from "./core/RenderPass";
export { Draw, type DrawOptions } from "./helper/Draw";
export { DepthDraw, type DepthDrawOptions } from "./helper/DepthDraw";
export {
  opaqueTriangles,
  depthOnlyTriangles,
  type OpaqueTrianglesOptions,
  type DepthOnlyTrianglesOptions,
  type OpaqueTrianglesState,
  type DepthOnlyTrianglesState,
} from "./helper/renderState";
export { ShadowMap, type ShadowMapOptions } from "./helper/ShadowMap";
export { wgslShadowPcf3x3 } from "./shader/shadow";
export { Compute, type ComputeOptions, type WorkgroupCount } from "./helper/Compute";
export { AxisHelper, type AxisHelperOptions } from "./helper/AxisHelper";
export { BallHelper, type BallHelperOptions, type BallDrawParams } from "./helper/BallHelper";
export { CopyHelper, type CopyHelperOptions } from "./helper/CopyHelper";
export { Texture2DPingPong, type Texture2DPingPongOptions } from "./helper/Texture2DPingPong";
export { Texture3DPingPong, type Texture3DPingPongOptions } from "./helper/Texture3DPingPong";
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
  createSceneTexture3DBindGroupLayout,
  createSceneTexture3DPipelineLayout,
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

export { EffectComposer, type EffectComposerOptions } from "./postprocessing/EffectComposer";
export { ShaderPass, type ShaderPassOptions } from "./postprocessing/ShaderPass";
export { createVignettePass } from "./postprocessing/passes/VignettePass";
export { createCurvePass } from "./postprocessing/passes/CurvePass";
export { createFXAAPass } from "./postprocessing/passes/FXAAPass";
export { createContrastBrightnessPass } from "./postprocessing/passes/ContrastBrightnessPass";
export { createHueSaturationPass } from "./postprocessing/passes/HueSaturationPass";
export { createGradientMapPass } from "./postprocessing/passes/GradientMapPass";

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
