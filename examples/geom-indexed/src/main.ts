import {
  assertWebGPUSupport,
  AxisHelper,
  beginRenderPass,
  BindGroup,
  Buffer,
  BufferUsage,
  createSceneUniformPipelineLayout,
  Device,
  Draw,
  Geom,
  Mesh,
  OrbitalControl,
  PerspectiveCamera,
} from "belfast";
import shaderCode from "./shaders/geom-indexed.wgsl?raw";

function setIdentity(dst: Float32Array, offset: number): void {
  dst.fill(0, offset, offset + 16);
  dst[offset + 0] = 1;
  dst[offset + 5] = 1;
  dst[offset + 10] = 1;
  dst[offset + 15] = 1;
}

async function main() {
  await assertWebGPUSupport();

  const canvas = document.createElement("canvas");
  canvas.style.cssText = "display:block;width:100vw;height:100vh;";
  document.body.appendChild(canvas);

  const device = await Device.create(canvas);
  const geom = Geom.cube({ size: 1.25 });

  const positionBuffer = Buffer.fromData(
    device,
    geom.positions,
    BufferUsage.vertex,
    "geom-positions",
  );
  const normalBuffer = Buffer.fromData(device, geom.normals, BufferUsage.vertex, "geom-normals");
  const mesh = new Mesh(geom.positions.length / 3)
    .addVertexBuffer({
      buffer: positionBuffer,
      arrayStride: 12,
      attributes: [{ shaderLocation: 0, format: "float32x3", offset: 0 }],
      slot: 0,
    })
    .addVertexBuffer({
      buffer: normalBuffer,
      arrayStride: 12,
      attributes: [{ shaderLocation: 1, format: "float32x3", offset: 0 }],
      slot: 1,
    });
  const indexBuffer = mesh.setIndexBufferFromData(device, geom.indices, "geom-indices");

  const sceneUniformData = new Float32Array(36);
  const uniformBuffer = Buffer.create(
    device,
    Buffer.uniformSize(sceneUniformData.byteLength),
    BufferUsage.uniform,
    "geom-scene-uniforms",
  );

  const camera = new PerspectiveCamera(Math.PI / 4, 1, 0.1, 100);
  const control = new OrbitalControl(camera, {
    listenerTarget: canvas,
    center: [0, 0, 0],
    radius: 3.25,
  });

  const { pipelineLayout, bindGroupLayout } = createSceneUniformPipelineLayout(
    device,
    "GeomIndexedScene",
  );

  const draw = new Draw(device, shaderCode, {
    label: "GeomIndexedCube",
    layout: pipelineLayout,
    vertexBuffers: mesh.getVertexLayouts(),
    primitive: { topology: "triangle-list", cullMode: "back" },
    depthStencil: {
      format: "depth24plus",
      depthWriteEnabled: true,
      depthCompare: "less",
    },
  });

  const axisHelper = new AxisHelper(device, { pipelineLayout });

  const bindGroup = BindGroup.create(
    device,
    bindGroupLayout,
    uniformBuffer,
    0,
    "geom-indexed-bind-group",
  );

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
    depthTexture = device.device.createTexture({
      label: "geom-indexed-depth",
      size: [width, height],
      format: "depth24plus",
      usage: GPUTextureUsage.RENDER_ATTACHMENT,
    });
    return depthTexture.createView();
  };

  const render = () => {
    device.resize();
    updateAspect();

    sceneUniformData.set(camera.getViewProjectionMatrix(), 0);
    setIdentity(sceneUniformData, 16);
    sceneUniformData[32] = -0.7;
    sceneUniformData[33] = -0.8;
    sceneUniformData[34] = -0.45;
    sceneUniformData[35] = 0;
    uniformBuffer.write(device, sceneUniformData);

    const encoder = device.device.createCommandEncoder();
    const pass = beginRenderPass(encoder, device.getCurrentTexture().createView(), {
      clearColor: { r: 0.01, g: 0.01, b: 0.015, a: 1 },
      depthStencilAttachment: {
        view: ensureDepthTexture(),
        depthLoadOp: "clear",
        depthClearValue: 1,
        depthStoreOp: "store",
      },
    });

    axisHelper.draw(pass, bindGroup);
    draw.draw(pass, mesh, bindGroup);
    pass.end();

    device.device.queue.submit([encoder.finish()]);
    requestAnimationFrame(render);
  };

  window.addEventListener("beforeunload", () => {
    control.destroy();
    axisHelper.destroy();
    positionBuffer.destroy();
    normalBuffer.destroy();
    indexBuffer.destroy();
    uniformBuffer.destroy();
    depthTexture?.destroy();
  });

  render();
}

main().catch((error) => {
  console.error(error);
});
