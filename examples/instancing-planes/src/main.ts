import {
  assertWebGPUSupport,
  AxisHelper,
  beginRenderPass,
  BindGroup,
  Buffer,
  BufferUsage,
  createPlaneTriangleList,
  createSceneUniformPipelineLayout,
  Device,
  Draw,
  Mesh,
  OrbitalControl,
  PerspectiveCamera,
} from "belfast";
import shaderCode from "./shaders/instanced-planes.wgsl?raw";

const INSTANCE_COUNT = 500_000;

function randomRange(min: number, max: number): number {
  return min + Math.random() * (max - min);
}

function buildInstanceData(): Float32Array {
  // vec4(position.xyz, size) + vec4(color.rgb, alpha)
  const data = new Float32Array(INSTANCE_COUNT * 8);
  for (let i = 0; i < INSTANCE_COUNT; i++) {
    const base = i * 8;
    data[base + 0] = randomRange(-20, 20);
    data[base + 1] = randomRange(-20, 20);
    data[base + 2] = randomRange(-20, 20);
    data[base + 3] = randomRange(0.05, 0.2);
    data[base + 4] = Math.random();
    data[base + 5] = Math.random();
    data[base + 6] = Math.random();
    data[base + 7] = 1;
  }
  return data;
}

async function main() {
  await assertWebGPUSupport();

  const canvas = document.createElement("canvas");
  canvas.style.cssText = "display:block;width:100vw;height:100vh;";
  document.body.appendChild(canvas);

  const device = await Device.create(canvas);
  const { positions } = createPlaneTriangleList(1, 1, 1, "xy");
  const vertexCount = positions.length / 3;
  const instanceData = buildInstanceData();

  const positionBuffer = Buffer.fromData(device, positions, BufferUsage.vertex, "plane-positions");
  const instanceBuffer = Buffer.fromData(
    device,
    instanceData,
    BufferUsage.vertex,
    "plane-instance-data",
  );

  const mesh = new Mesh(vertexCount)
    .addVertexBuffer({
      buffer: positionBuffer,
      arrayStride: 12,
      attributes: [{ shaderLocation: 0, format: "float32x3", offset: 0 }],
      slot: 0,
      stepMode: "vertex",
    })
    .addVertexBuffer({
      buffer: instanceBuffer,
      arrayStride: 32,
      attributes: [
        { shaderLocation: 1, format: "float32x4", offset: 0 },
        { shaderLocation: 2, format: "float32x4", offset: 16 },
      ],
      slot: 1,
      stepMode: "instance",
    });

  const uniformBuffer = Buffer.create(
    device,
    Buffer.uniformSize(PerspectiveCamera.uniformByteSize()),
    BufferUsage.uniform,
    "camera-uniforms",
  );
  const cameraUniformData = new Float32Array(PerspectiveCamera.uniformFloatCount);

  const camera = new PerspectiveCamera(Math.PI / 4, 1, 0.1, 200);
  const control = new OrbitalControl(camera, {
    listenerTarget: canvas,
    center: [0, 0, 0],
    radius: 60,
  });

  const { pipelineLayout, bindGroupLayout } = createSceneUniformPipelineLayout(
    device,
    "InstancingScene",
  );

  const draw = new Draw(device, shaderCode, {
    label: "InstancedPlanes",
    layout: pipelineLayout,
    vertexBuffers: mesh.getVertexLayouts(),
    primitive: { topology: "triangle-list", cullMode: "back" },
    depthStencil: {
      format: "depth24plus",
      depthWriteEnabled: true,
      depthCompare: "less",
    },
  });

  const axes = new AxisHelper(device, { pipelineLayout });
  const bindGroup = BindGroup.create(device, bindGroupLayout, uniformBuffer, 0, "scene-bind-group");

  window.addEventListener("beforeunload", () => {
    control.destroy();
    axes.destroy();
    positionBuffer.destroy();
    instanceBuffer.destroy();
    uniformBuffer.destroy();
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

    camera.writeUniformData(cameraUniformData);
    uniformBuffer.write(device, cameraUniformData);

    const colorView = device.getCurrentTexture().createView();
    const depthView = ensureDepthTexture();
    const encoder = device.device.createCommandEncoder();
    const pass = beginRenderPass(encoder, colorView, {
      depthStencilAttachment: {
        view: depthView,
        depthLoadOp: "clear",
        depthClearValue: 1,
        depthStoreOp: "store",
      },
    });

    axes.draw(pass, bindGroup);
    draw.draw(pass, mesh, bindGroup, INSTANCE_COUNT);

    pass.end();
    device.device.queue.submit([encoder.finish()]);
    requestAnimationFrame(render);
  };

  render();
}

main().catch((error) => {
  console.error(error);
});
