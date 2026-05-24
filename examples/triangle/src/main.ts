import {
  assertWebGPUSupport,
  beginRenderPass,
  Device,
  Draw,
} from "belfast";
import shaderCode from "./shaders/triangle.wgsl?raw";

async function main() {
  await assertWebGPUSupport();

  const canvas = document.createElement("canvas");
  canvas.style.cssText = "display:block;width:100vw;height:100vh;";
  document.body.appendChild(canvas);

  const device = await Device.create(canvas);
  const draw = new Draw(device, shaderCode, "Triangle");

  const render = () => {
    device.resize();
    const textureView = device.getCurrentTexture().createView();
    const encoder = device.device.createCommandEncoder();
    const pass = beginRenderPass(encoder, textureView);
    draw.draw(pass);
    pass.end();
    device.device.queue.submit([encoder.finish()]);
    requestAnimationFrame(render);
  };

  render();
}

main().catch((error) => {
  console.error(error);
});
