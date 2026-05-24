import type { Device } from "../core/Device";

/** `@group(0) @binding(0) var<uniform>` with a single mat4x4 view-projection matrix. */
export function createSceneUniformBindGroupLayout(
  device: Device,
  label = "SceneUniformBindGroupLayout",
): GPUBindGroupLayout {
  return device.device.createBindGroupLayout({
    label,
    entries: [
      {
        binding: 0,
        visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT,
        buffer: { type: "uniform" },
      },
    ],
  });
}

/**
 * Pipeline layout for draws that use `SceneUniforms { viewProj: mat4x4<f32> }`.
 * Pass `pipelineLayout` into `Draw` / `AxisHelper` so one bind group works across pipelines.
 */
export function createSceneUniformPipelineLayout(
  device: Device,
  label = "SceneUniformPipelineLayout",
): {
  pipelineLayout: GPUPipelineLayout;
  bindGroupLayout: GPUBindGroupLayout;
} {
  const bindGroupLayout = createSceneUniformBindGroupLayout(device, `${label}BindGroup`);
  const pipelineLayout = device.device.createPipelineLayout({
    label,
    bindGroupLayouts: [bindGroupLayout],
  });
  return { pipelineLayout, bindGroupLayout };
}

/** `@group(1) @binding(0)` per-draw ball instance (translate, scale, color, opacity). */
export function createBallInstanceBindGroupLayout(
  device: Device,
  label = "BallInstanceBindGroupLayout",
): GPUBindGroupLayout {
  return device.device.createBindGroupLayout({
    label,
    entries: [
      {
        binding: 0,
        visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT,
        buffer: { type: "uniform" },
      },
    ],
  });
}

/**
 * Two-group pipeline: group 0 = scene `viewProj`, group 1 = ball instance uniforms.
 * Group 0 layout matches `createSceneUniformBindGroupLayout` for shared camera bind groups.
 */
export function createSceneBallPipelineLayout(
  device: Device,
  label = "SceneBallPipelineLayout",
): {
  pipelineLayout: GPUPipelineLayout;
  sceneBindGroupLayout: GPUBindGroupLayout;
  ballBindGroupLayout: GPUBindGroupLayout;
} {
  const sceneBindGroupLayout = createSceneUniformBindGroupLayout(device, `${label}Scene`);
  const ballBindGroupLayout = createBallInstanceBindGroupLayout(device, `${label}Ball`);
  const pipelineLayout = device.device.createPipelineLayout({
    label,
    bindGroupLayouts: [sceneBindGroupLayout, ballBindGroupLayout],
  });
  return { pipelineLayout, sceneBindGroupLayout, ballBindGroupLayout };
}
