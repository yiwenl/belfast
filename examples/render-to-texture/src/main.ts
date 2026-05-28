import {
  assertWebGPUSupport,
  beginRenderPass,
  BindGroup,
  Buffer,
  BufferUsage,
  CopyHelper,
  createSceneUniformPipelineLayout,
  Device,
  Draw,
  Geom,
  Mesh,
  OrbitalControl,
  PerspectiveCamera,
  RenderTarget,
} from "belfast";
import shaderCode from "./shaders/cube-lit.wgsl?raw";

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
  const cube = Geom.cube({ size: 1 });
  const vertexCount = cube.positions.length / 3;

  const positionBuffer = Buffer.fromData(
    device,
    cube.positions,
    BufferUsage.vertex,
    "cube-positions",
  );
  const normalBuffer = Buffer.fromData(device, cube.normals, BufferUsage.vertex, "cube-normals");

  const mesh = new Mesh(vertexCount)
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
  const indexBuffer = mesh.setIndexBufferFromData(device, cube.indices, "cube-indices");

  const sceneUniformData = new Float32Array(36);
  const uniformBuffer = Buffer.create(
    device,
    Buffer.uniformSize(sceneUniformData.byteLength),
    BufferUsage.uniform,
    "cube-scene-uniforms",
  );

  const camera = new PerspectiveCamera(Math.PI / 4, 1, 0.1, 100);
  const control = new OrbitalControl(camera, {
    listenerTarget: canvas,
    center: [0, 0, 0],
    radius: 3,
  });

  const { pipelineLayout, bindGroupLayout } = createSceneUniformPipelineLayout(
    device,
    "RenderToTextureScene",
  );

  const cubeDraw = new Draw(device, shaderCode, {
    label: "LitCube",
    layout: pipelineLayout,
    vertexBuffers: mesh.getVertexLayouts(),
    depthStencil: {
      format: "depth24plus",
      depthWriteEnabled: true,
      depthCompare: "less",
    },
  });

  const bindGroup = BindGroup.create(
    device,
    bindGroupLayout,
    uniformBuffer,
    0,
    "cube-scene-bind-group",
  );

  const renderTarget = RenderTarget.create(device, {
    width: 1,
    height: 1,
    withDepth: true,
    label: "OffscreenTarget",
  });
  const copyHelper = new CopyHelper(device, { label: "CopyToScreen" });

  window.addEventListener("beforeunload", () => {
    control.destroy();
    positionBuffer.destroy();
    normalBuffer.destroy();
    indexBuffer.destroy();
    uniformBuffer.destroy();
    renderTarget.destroy();
    screenDepthTexture?.destroy();
  });

  let lastWidth = 0;
  let lastHeight = 0;
  let screenDepthTexture: GPUTexture | null = null;

  const updateAspect = () => {
    if (canvas.width === lastWidth && canvas.height === lastHeight) {
      return;
    }
    lastWidth = canvas.width;
    lastHeight = canvas.height;
    if (lastWidth > 0 && lastHeight > 0) {
      camera.setAspect(lastWidth / lastHeight);
    }
    renderTarget.resize(lastWidth, lastHeight);
    screenDepthTexture?.destroy();
    screenDepthTexture = device.device.createTexture({
      label: "screen-depth-texture",
      size: [Math.max(1, lastWidth), Math.max(1, lastHeight)],
      format: "depth24plus",
      usage: GPUTextureUsage.RENDER_ATTACHMENT,
    });
  };

  const render = () => {
    device.resize();
    updateAspect();

    sceneUniformData.set(camera.getViewProjectionMatrix(), 0);
    setIdentity(sceneUniformData, 16);
    sceneUniformData[32] = -0.6;
    sceneUniformData[33] = -0.7;
    sceneUniformData[34] = -0.4;
    sceneUniformData[35] = 0;
    uniformBuffer.write(device, sceneUniformData);

    const encoder = device.device.createCommandEncoder();

    const offscreenPass = renderTarget.beginRenderPass(encoder, {
      clearColor: { r: 0.03, g: 0.03, b: 0.06, a: 1 },
    });
    cubeDraw.draw(offscreenPass, mesh, bindGroup);
    offscreenPass.end();

    const screenView = device.getCurrentTexture().createView();

    const screenCubePass = beginRenderPass(encoder, screenView, {
      clearColor: { r: 0, g: 0, b: 0, a: 1 },
      depthStencilAttachment: {
        view: screenDepthTexture!.createView(),
        depthLoadOp: "clear",
        depthClearValue: 1,
        depthStoreOp: "store",
      },
    });
    // Draw the lit cube directly to the screen in the second pass.
    cubeDraw.draw(screenCubePass, mesh, bindGroup);
    screenCubePass.end();

    // Copy pass overlays texture preview without depth-state requirements.
    const screenCopyPass = beginRenderPass(encoder, screenView, {
      loadOp: "load",
    });
    // Then draw the offscreen texture as a small preview in the bottom-left corner.
    copyHelper.draw(screenCopyPass, renderTarget.colorView, renderTarget.sampler, {
      x: 0,
      y: Math.max(0, canvas.height - Math.floor(canvas.height * 0.32)),
      width: Math.floor(canvas.width * 0.32),
      height: Math.floor(canvas.height * 0.32),
    });
    screenCopyPass.end();

    device.device.queue.submit([encoder.finish()]);
    requestAnimationFrame(render);
  };

  render();
}

main().catch((error) => {
  console.error(error);
});
