import {
  assertWebGPUSupport,
  beginRenderPass,
  Buffer,
  BufferUsage,
  Device,
  Draw,
  Mesh,
} from "belfast";
import shaderCode from "./shaders/triangle.wgsl?raw";

const positions = new Float32Array([0.0, 0.5, -0.5, -0.5, 0.5, -0.5]);
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
      arrayStride: 8,
      attributes: [{ shaderLocation: 0, format: "float32x2", offset: 0 }],
      slot: 0,
    })
    .addVertexBuffer({
      buffer: colorBuffer,
      arrayStride: 12,
      attributes: [{ shaderLocation: 1, format: "float32x3", offset: 0 }],
      slot: 1,
    });

  const draw = new Draw(device, shaderCode, {
    label: "Triangle",
    vertexBuffers: mesh.getVertexLayouts(),
  });

  const render = () => {
    device.resize();
    const textureView = device.getCurrentTexture().createView();
    const encoder = device.device.createCommandEncoder();
    const pass = beginRenderPass(encoder, textureView);
    draw.draw(pass, mesh);
    pass.end();
    device.device.queue.submit([encoder.finish()]);
    requestAnimationFrame(render);
  };

  render();
}

main().catch((error) => {
  console.error(error);
});
