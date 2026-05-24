import {
  assertWebGPUSupport,
  AxisHelper,
  beginRenderPass,
  BindGroup,
  Buffer,
  BufferUsage,
  createSceneTexturePipelineLayout,
  Device,
  Draw,
  Mesh,
  OrbitalControl,
  PerspectiveCamera,
  Texture,
  createPlaneTriangleList,
} from "belfast";
import shaderCode from "./shaders/textured-plane.wgsl?raw";

async function main() {
  await assertWebGPUSupport();

  const canvas = document.createElement("canvas");
  canvas.style.cssText = "display:block;width:100vw;height:100vh;";
  document.body.appendChild(canvas);

  const device = await Device.create(canvas);
  const texture = await Texture.load(device, "/image.jpg", { label: "landscape-photo" });

  const maxSize = 1.5;
  const aspect = texture.width / texture.height;
  const planeW = aspect >= 1 ? maxSize : maxSize * aspect;
  const planeH = aspect >= 1 ? maxSize / aspect : maxSize;

  const { positions, uvs } = createPlaneTriangleList(planeW, planeH, 1, "xy");
  const vertexCount = positions.length / 3;
  const { vertex } = BufferUsage;

  const positionBuffer = Buffer.fromData(device, positions, vertex, "plane-positions");
  const uvBuffer = Buffer.fromData(device, uvs, vertex, "plane-uvs");

  const mesh = new Mesh(vertexCount)
    .addVertexBuffer({
      buffer: positionBuffer,
      arrayStride: 12,
      attributes: [{ shaderLocation: 0, format: "float32x3", offset: 0 }],
      slot: 0,
    })
    .addVertexBuffer({
      buffer: uvBuffer,
      arrayStride: 8,
      attributes: [{ shaderLocation: 1, format: "float32x2", offset: 0 }],
      slot: 1,
    });

  const uniformBuffer = Buffer.create(
    device,
    Buffer.uniformSize(64),
    BufferUsage.uniform,
    "camera-uniforms",
  );

  const camera = new PerspectiveCamera(Math.PI / 4, 1, 0.1, 100);
  const control = new OrbitalControl(camera, {
    listenerTarget: canvas,
    center: [0, 0, 0],
    radius: 2.5,
  });

  const { pipelineLayout, bindGroupLayout } = createSceneTexturePipelineLayout(
    device,
    "TextureScene",
  );

  const draw = new Draw(device, shaderCode, {
    label: "TexturedPlane",
    layout: pipelineLayout,
    vertexBuffers: mesh.getVertexLayouts(),
    primitive: { topology: "triangle-list", cullMode: "none" },
    depthStencil: {
      format: "depth24plus",
      depthWriteEnabled: true,
      depthCompare: "less",
    },
  });

  const axes = new AxisHelper(device, { pipelineLayout });

  const bindGroup = BindGroup.create(
    device,
    bindGroupLayout,
    [
      { binding: 0, resource: uniformBuffer },
      { binding: 1, resource: texture.view },
      { binding: 2, resource: texture.sampler },
    ],
    "texture-scene-bind-group",
  );

  window.addEventListener("beforeunload", () => {
    control.destroy();
    axes.destroy();
    texture.destroy();
    positionBuffer.destroy();
    uvBuffer.destroy();
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
    depthTexture = device.device.createTexture({
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

    const viewProj = camera.getViewProjectionMatrix();
    uniformBuffer.write(device, viewProj);

    const textureView = device.getCurrentTexture().createView();
    const depthView = ensureDepthTexture();
    const encoder = device.device.createCommandEncoder();
    const pass = beginRenderPass(encoder, textureView, {
      depthStencilAttachment: {
        view: depthView,
        depthLoadOp: "clear",
        depthClearValue: 1,
        depthStoreOp: "store",
      },
    });
    axes.draw(pass, bindGroup);
    draw.draw(pass, mesh, bindGroup);
    pass.end();
    device.device.queue.submit([encoder.finish()]);
    requestAnimationFrame(render);
  };

  render();
}

main().catch((error) => {
  console.error(error);
});
