import {
  assertWebGPUSupport,
  AxisHelper,
  beginRenderPass,
  BindGroup,
  Buffer,
  BufferUsage,
  Compute,
  createSceneTexture3DPipelineLayout,
  Device,
  Draw,
  Mesh,
  OrbitalControl,
  PerspectiveCamera,
  Texture3D,
  UniformBlock,
} from "belfast";
import computeShaderCode from "./shaders/curl-noise-compute.wgsl?raw";
import fieldLinesShaderCode from "./shaders/field-lines.wgsl?raw";

const TEX_SIZE = 32;
const VIS_GRID = 16;
const WORKGROUP_SIZE = 4;
const VOLUME_EXTENT = 2.0;
const ARROW_SCALE = 0.12;
const INSTANCE_COUNT = VIS_GRID * VIS_GRID * VIS_GRID;

const computeUniforms = UniformBlock.create({
  time: "f32",
  curlScale: "f32",
  gridSize: "f32",
});

const sceneUniforms = UniformBlock.create({
  viewProj: "mat4x4f",
  arrowScale: "f32",
  visGrid: "f32",
  texSize: "f32",
  volumeExtent: "f32",
});

async function main() {
  await assertWebGPUSupport();

  const canvas = document.createElement("canvas");
  canvas.style.cssText = "display:block;width:100vw;height:100vh;";
  document.body.appendChild(canvas);

  const device = await Device.create(canvas);
  const fieldTexture = Texture3D.create(device, TEX_SIZE, { label: "CurlNoiseField" });

  const segmentPositions = new Float32Array([0, 0, 0, 0, 1, 0]);
  const segmentBuffer = Buffer.fromData(
    device,
    segmentPositions,
    BufferUsage.vertex,
    "line-segment",
  );

  const mesh = new Mesh(2).addVertexBuffer({
    buffer: segmentBuffer,
    arrayStride: 12,
    attributes: [{ shaderLocation: 0, format: "float32x3", offset: 0 }],
    slot: 0,
    stepMode: "vertex",
  });

  const simUniformBuffer = Buffer.create(
    device,
    Buffer.uniformSize(computeUniforms.byteSize),
    BufferUsage.uniform,
    "curl-sim-uniforms",
  );

  const sceneUniformBuffer = Buffer.create(
    device,
    Buffer.uniformSize(sceneUniforms.byteSize),
    BufferUsage.uniform,
    "field-scene-uniforms",
  );

  const camera = new PerspectiveCamera(Math.PI / 4, 1, 0.1, 100);
  const control = new OrbitalControl(camera, {
    listenerTarget: canvas,
    center: [0, 0, 0],
    radius: 3.5,
  });

  const compute = new Compute(device, computeShaderCode, {
    label: "CurlNoiseCompute",
    entryPoint: "cs_main",
  });

  const computeBindGroup = BindGroup.create(
    device,
    compute.getBindGroupLayout(0),
    [
      { binding: 0, resource: simUniformBuffer },
      { binding: 1, resource: fieldTexture.storageView },
    ],
    "curl-noise-compute-bind-group",
  );

  const { pipelineLayout, bindGroupLayout } = createSceneTexture3DPipelineLayout(
    device,
    "FieldLinesScene",
  );

  const fieldDraw = new Draw(device, fieldLinesShaderCode, {
    label: "FieldLines",
    layout: pipelineLayout,
    vertexBuffers: mesh.getVertexLayouts(),
    primitive: { topology: "line-list", cullMode: "none" },
    depthStencil: {
      format: "depth24plus",
      depthWriteEnabled: true,
      depthCompare: "less",
    },
  });

  const axes = new AxisHelper(device, { pipelineLayout, length: 1.2 });

  const sceneBindGroup = BindGroup.create(
    device,
    bindGroupLayout,
    [
      { binding: 0, resource: sceneUniformBuffer },
      { binding: 1, resource: fieldTexture.view },
      { binding: 2, resource: fieldTexture.sampler },
    ],
    "field-scene-bind-group",
  );

  const dispatchSize = TEX_SIZE / WORKGROUP_SIZE;

  window.addEventListener("beforeunload", () => {
    control.destroy();
    axes.destroy();
    fieldTexture.destroy();
    segmentBuffer.destroy();
    simUniformBuffer.destroy();
    sceneUniformBuffer.destroy();
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

    const time = performance.now() * 0.001;

    computeUniforms
      .set("time", time)
      .set("curlScale", 0.35)
      .set("gridSize", TEX_SIZE)
      .writeToBuffer(simUniformBuffer, device);

    sceneUniforms
      .set("viewProj", camera.getViewProjectionMatrix())
      .set("arrowScale", ARROW_SCALE)
      .set("visGrid", VIS_GRID)
      .set("texSize", TEX_SIZE)
      .set("volumeExtent", VOLUME_EXTENT)
      .writeToBuffer(sceneUniformBuffer, device);

    const colorView = device.getCurrentTexture().createView();
    const depthView = ensureDepthTexture();
    const encoder = device.gpu.createCommandEncoder({ label: "curl-noise-frame" });

    compute.run(
      encoder,
      computeBindGroup,
      [dispatchSize, dispatchSize, dispatchSize],
      "curl-noise-compute",
    );

    const pass = beginRenderPass(encoder, colorView, {
      clearColor: { r: 0.04, g: 0.04, b: 0.07, a: 1 },
      depthStencilAttachment: {
        view: depthView,
        depthLoadOp: "clear",
        depthClearValue: 1,
        depthStoreOp: "store",
      },
    });

    fieldDraw.draw(pass, mesh, sceneBindGroup, INSTANCE_COUNT);
    axes.draw(pass, sceneBindGroup);
    pass.end();

    device.gpu.queue.submit([encoder.finish()]);
    requestAnimationFrame(render);
  };

  render();
}

main().catch((error) => {
  console.error(error);
});
