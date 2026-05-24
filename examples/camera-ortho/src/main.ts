import {
  assertWebGPUSupport,
  beginRenderPass,
  BindGroup,
  Buffer,
  BufferUsage,
  Device,
  Draw,
  Mesh,
  OrthographicCamera,
} from "belfast";
import shaderCode from "./shaders/triangle.wgsl?raw";

const positions = new Float32Array([0.0, 0.5, 0.0, -0.5, -0.5, 0.0, 0.5, -0.5, 0.0]);
const colors = new Float32Array([1, 0, 0, 0, 1, 0, 0, 0, 1]);

async function main() {
  await assertWebGPUSupport();

  const canvas = document.createElement("canvas");
  canvas.style.cssText = "display:block;width:100vw;height:100vh;";
  document.body.appendChild(canvas);

  const device = await Device.create(canvas);
  const { vertex } = BufferUsage;

  const positionBuffer = Buffer.fromData(device, positions, vertex, "triangle-positions");
  const colorBuffer = Buffer.fromData(device, colors, vertex, "triangle-colors");

  const mesh = new Mesh(3)
    .addVertexBuffer({
      buffer: positionBuffer,
      arrayStride: 12,
      attributes: [{ shaderLocation: 0, format: "float32x3", offset: 0 }],
      slot: 0,
    })
    .addVertexBuffer({
      buffer: colorBuffer,
      arrayStride: 12,
      attributes: [{ shaderLocation: 1, format: "float32x3", offset: 0 }],
      slot: 1,
    });

  const uniformBuffer = Buffer.create(
    device,
    Buffer.uniformSize(64),
    BufferUsage.uniform,
    "camera-uniforms",
  );

  const camera = new OrthographicCamera(-1, 1, -1, 1, 0.1, 100);
  camera.lookAt([0, 0, 3], [0, 0, 0]);

  const draw = new Draw(device, shaderCode, {
    label: "OrthoTriangle",
    vertexBuffers: mesh.getVertexLayouts(),
    depthStencil: {
      format: "depth24plus",
      depthWriteEnabled: true,
      depthCompare: "less",
    },
  });

  const bindGroup = BindGroup.create(
    device,
    draw.getBindGroupLayout(0),
    uniformBuffer,
    0,
    "camera-bind-group",
  );

  let depthTexture: GPUTexture | null = null;
  let lastWidth = 0;
  let lastHeight = 0;

  const updateFrustum = () => {
    if (canvas.width === lastWidth && canvas.height === lastHeight) {
      return;
    }
    lastWidth = canvas.width;
    lastHeight = canvas.height;
    if (lastWidth === 0 || lastHeight === 0) {
      return;
    }
    const aspect = lastWidth / lastHeight;
    const halfHeight = 1;
    camera.setOrthographic(
      -halfHeight * aspect,
      halfHeight * aspect,
      -halfHeight,
      halfHeight,
      0.1,
      100,
    );
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
    updateFrustum();

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
