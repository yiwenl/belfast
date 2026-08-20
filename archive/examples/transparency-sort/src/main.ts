import {
  assertWebGPUSupport,
  AxisHelper,
  beginRenderPass,
  BindGroup,
  Buffer,
  BufferUsage,
  Compute,
  createPlaneTriangleList,
  createSceneUniformPipelineLayout,
  Device,
  Draw,
  Mesh,
  OrbitalControl,
  PerspectiveCamera,
} from "belfast";
import distanceShaderCode from "./shaders/planes-distance.wgsl?raw";
import sortShaderCode from "./shaders/bitonic-sort.wgsl?raw";
import drawShaderCode from "./shaders/planes-draw.wgsl?raw";

const INSTANCE_COUNT = 1000;
/** Bitonic sort needs a power-of-two element count; pad to 1024 and ignore the tail. */
const SORT_COUNT = 1024;
const WORKGROUP_SIZE = 256;
const SORT_WORKGROUPS = Math.ceil(SORT_COUNT / WORKGROUP_SIZE);

/** WGSL `Plane`: 2× vec4 (posSize + color) = 32 bytes, shared by compute + draw. */
const PLANE_FLOATS = 8;
/** WGSL `Key`: f32 dist + u32 index = 8 bytes. */
const KEY_BYTES = 8;
const SPREAD = 11;

function randomRange(min: number, max: number): number {
  return min + Math.random() * (max - min);
}

function buildPlaneData(): Float32Array {
  const data = new Float32Array(INSTANCE_COUNT * PLANE_FLOATS);
  for (let i = 0; i < INSTANCE_COUNT; i++) {
    const base = i * PLANE_FLOATS;
    data[base + 0] = randomRange(-SPREAD, SPREAD);
    data[base + 1] = randomRange(-SPREAD, SPREAD);
    data[base + 2] = randomRange(-SPREAD, SPREAD);
    data[base + 3] = randomRange(1.4, 3.2); // half-size

    const hue = Math.random();
    data[base + 4] = 0.5 + 0.5 * Math.cos(hue * 6.2831853);
    data[base + 5] = 0.5 + 0.5 * Math.cos(hue * 6.2831853 + 2.0944);
    data[base + 6] = 0.5 + 0.5 * Math.cos(hue * 6.2831853 + 4.1888);
    data[base + 7] = randomRange(0.25, 0.55); // alpha
  }
  return data;
}

/** Fixed (k, j) schedule of bitonic merge steps for a power-of-two array. */
function buildBitonicSchedule(count: number): { k: number; j: number }[] {
  const passes: { k: number; j: number }[] = [];
  for (let k = 2; k <= count; k <<= 1) {
    for (let j = k >> 1; j > 0; j >>= 1) {
      passes.push({ k, j });
    }
  }
  return passes;
}

async function main() {
  await assertWebGPUSupport();

  const canvas = document.createElement("canvas");
  canvas.style.cssText = "display:block;width:100vw;height:100vh;";
  document.body.appendChild(canvas);

  const device = await Device.create(canvas);

  const { positions } = createPlaneTriangleList(1, 1, 1, "xy");
  const vertexCount = positions.length / 3;
  const positionBuffer = Buffer.fromData(device, positions, BufferUsage.vertex, "plane-positions");

  const planeBuffer = Buffer.fromData(
    device,
    buildPlaneData(),
    BufferUsage.storage,
    "plane-instance-data",
  );
  // Sorted draw order: keys[i] = { dist, index } produced by the compute passes.
  const keysBuffer = Buffer.create(
    device,
    SORT_COUNT * KEY_BYTES,
    BufferUsage.storage,
    "sort-keys",
  );

  const mesh = new Mesh(vertexCount).addVertexBuffer({
    buffer: positionBuffer,
    arrayStride: 12,
    attributes: [{ shaderLocation: 0, format: "float32x3", offset: 0 }],
    slot: 0,
    stepMode: "vertex",
  });

  // Camera / scene uniforms (viewProj + camera basis), shared with the axis helper.
  const uniformBuffer = Buffer.create(
    device,
    Buffer.uniformSize(PerspectiveCamera.uniformByteSize()),
    BufferUsage.uniform,
    "camera-uniforms",
  );
  const cameraUniformData = new Float32Array(PerspectiveCamera.uniformFloatCount);

  const RAD = Math.PI / 180;
  const camera = new PerspectiveCamera(55 * RAD, 1, 0.1, 200);
  const control = new OrbitalControl(camera, {
    listenerTarget: canvas,
    center: [0, 0, 0],
    radius: 42,
  });

  // ---- Compute: distance seeding + bitonic sort -------------------------------

  // DistParams: vec4 cameraPos + (count, total, pad, pad) = 32 bytes.
  const distParamsBuffer = Buffer.create(device, 32, BufferUsage.uniform, "distance-params");
  const distParamsData = new ArrayBuffer(32);
  const distParamsF32 = new Float32Array(distParamsData, 0, 4);
  const distParamsU32 = new Uint32Array(distParamsData, 16, 4);
  distParamsU32[0] = INSTANCE_COUNT;
  distParamsU32[1] = SORT_COUNT;

  const distanceCompute = new Compute(device, distanceShaderCode, {
    label: "PlaneDistance",
    entryPoint: "cs_main",
  });
  const distanceBindGroup = BindGroup.create(
    device,
    distanceCompute.getBindGroupLayout(0),
    [
      { binding: 0, resource: distParamsBuffer },
      { binding: 1, resource: planeBuffer },
      { binding: 2, resource: keysBuffer },
    ],
    "distance-bind-group",
  );

  const sortCompute = new Compute(device, sortShaderCode, {
    label: "BitonicSort",
    entryPoint: "cs_main",
  });

  // Each (k, j) step is constant across frames, so build its uniform + bind group once.
  const schedule = buildBitonicSchedule(SORT_COUNT);
  const sortBindGroups = schedule.map(({ k, j }, index) => {
    const paramsBuffer = Buffer.create(device, 16, BufferUsage.uniform, `sort-params-${index}`);
    paramsBuffer.write(device, new Uint32Array([j, k, SORT_COUNT, 0]));
    return BindGroup.create(
      device,
      sortCompute.getBindGroupLayout(0),
      [
        { binding: 0, resource: paramsBuffer },
        { binding: 1, resource: keysBuffer },
      ],
      `sort-bind-group-${index}`,
    );
  });

  // ---- Render -----------------------------------------------------------------

  const scene = createSceneUniformPipelineLayout(device, "TransparencySortScene");

  const storageBindGroupLayout = device.gpu.createBindGroupLayout({
    label: "PlanesStorageLayout",
    entries: [
      { binding: 0, visibility: GPUShaderStage.VERTEX, buffer: { type: "read-only-storage" } },
      { binding: 1, visibility: GPUShaderStage.VERTEX, buffer: { type: "read-only-storage" } },
    ],
  });
  const planesPipelineLayout = device.gpu.createPipelineLayout({
    label: "PlanesPipelineLayout",
    bindGroupLayouts: [scene.bindGroupLayout, storageBindGroupLayout],
  });

  const draw = new Draw(device, drawShaderCode, {
    label: "TransparencyPlanes",
    layout: planesPipelineLayout,
    vertexBuffers: mesh.getVertexLayouts(),
    primitive: { topology: "triangle-list", cullMode: "none" },
    depthStencil: {
      format: "depth24plus",
      depthWriteEnabled: false,
      depthCompare: "less",
    },
    targets: [
      {
        format: device.format,
        blend: {
          color: { srcFactor: "src-alpha", dstFactor: "one-minus-src-alpha", operation: "add" },
          alpha: { srcFactor: "one", dstFactor: "one-minus-src-alpha", operation: "add" },
        },
      },
    ],
  });

  const axes = new AxisHelper(device, { pipelineLayout: scene.pipelineLayout, length: 50 });
  const sceneBindGroup = BindGroup.create(
    device,
    scene.bindGroupLayout,
    uniformBuffer,
    0,
    "scene-bind-group",
  );
  const storageBindGroup = BindGroup.create(
    device,
    storageBindGroupLayout,
    [
      { binding: 0, resource: planeBuffer },
      { binding: 1, resource: keysBuffer },
    ],
    "planes-storage-bind-group",
  );

  window.addEventListener("beforeunload", () => {
    control.destroy();
    axes.destroy();
    positionBuffer.destroy();
    planeBuffer.destroy();
    keysBuffer.destroy();
    uniformBuffer.destroy();
    distParamsBuffer.destroy();
  });

  let depthTexture: GPUTexture | null = null;
  let lastWidth = 0;
  let lastHeight = 0;

  const updateAspect = () => {
    if (canvas.width === lastWidth && canvas.height === lastHeight) {
      return;
    }
    lastWidth = canvas.width;
    lastHeight = canvas.height;
    if (lastWidth > 0 && lastHeight > 0) {
      camera.setAspect(lastWidth / lastHeight);
    }
  };

  const ensureDepthTexture = () => {
    const width = canvas.width;
    const height = canvas.height;
    if (depthTexture && depthTexture.width === width && depthTexture.height === height) {
      return depthTexture.createView();
    }
    depthTexture?.destroy();
    depthTexture = device.gpu.createTexture({
      label: "depth-texture",
      size: [width, height],
      format: "depth24plus",
      usage: GPUTextureUsage.RENDER_ATTACHMENT,
    });
    return depthTexture.createView();
  };

  const render = () => {
    device.resize();
    updateAspect();

    camera.writeUniformData(cameraUniformData);
    uniformBuffer.write(device, cameraUniformData);

    const cameraPos = camera.getPosition();
    distParamsF32[0] = cameraPos[0];
    distParamsF32[1] = cameraPos[1];
    distParamsF32[2] = cameraPos[2];
    distParamsBuffer.write(device, distParamsData);

    const colorView = device.getCurrentTexture().createView();
    const depthView = ensureDepthTexture();
    const encoder = device.gpu.createCommandEncoder({ label: "transparency-sort-frame" });

    // Re-sort every frame: seed distances, then run the full bitonic schedule.
    // Successive dispatches in one pass are auto-synchronized on `keysBuffer`.
    const computePass = encoder.beginComputePass({ label: "sort-planes" });
    distanceCompute.dispatch(computePass, distanceBindGroup, SORT_WORKGROUPS);
    for (const bindGroup of sortBindGroups) {
      sortCompute.dispatch(computePass, bindGroup, SORT_WORKGROUPS);
    }
    computePass.end();

    const pass = beginRenderPass(encoder, colorView, {
      clearColor: { r: 0.03, g: 0.04, b: 0.06, a: 1 },
      depthStencilAttachment: {
        view: depthView,
        depthLoadOp: "clear",
        depthClearValue: 1,
        depthStoreOp: "store",
      },
    });

    axes.draw(pass, sceneBindGroup);
    draw.draw(pass, mesh, [sceneBindGroup, storageBindGroup], INSTANCE_COUNT);

    pass.end();
    device.gpu.queue.submit([encoder.finish()]);
    requestAnimationFrame(render);
  };

  render();
}

main().catch((error) => {
  console.error(error);
});
