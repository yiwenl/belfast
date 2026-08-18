import { Buffer, BufferUsage, Device, Draw, Mesh } from "belfast-wasm";

import type { WebExample } from "../main";
import shaderCode from "../shaders/colored-triangle.wgsl?raw";

const positions = new Float32Array([0.0, 0.6, -0.6, -0.5, 0.6, -0.5]);

const colors = new Float32Array([1.0, 0.2, 0.15, 0.15, 0.95, 0.35, 0.2, 0.4, 1.0]);

export const coloredTriangle: WebExample = async (canvas, reportError) => {
  const device = await Device.create(canvas);
  const positionBuffer = Buffer.fromData(
    device,
    positions,
    BufferUsage.vertex,
    "ColoredTrianglePositions",
  );
  const colorBuffer = Buffer.fromData(device, colors, BufferUsage.vertex, "ColoredTriangleColors");
  const mesh = new Mesh(3)
    .addVertexBuffer(positionBuffer, {
      arrayStride: 8,
      attributes: [{ shaderLocation: 0, format: "float32x2", offset: 0 }],
      slot: 0,
    })
    .addVertexBuffer(colorBuffer, {
      arrayStride: 12,
      attributes: [{ shaderLocation: 1, format: "float32x3", offset: 0 }],
      slot: 1,
    });
  const draw = new Draw(device, shaderCode, mesh, { label: "ColoredTriangle" });

  let frame = 0;
  let stopped = false;

  const render = () => {
    if (stopped) {
      return;
    }

    try {
      if (device.resize()) {
        device.render(draw, mesh);
      }
    } catch (error) {
      stopped = true;
      reportError(error);
      return;
    }

    frame = requestAnimationFrame(render);
  };

  frame = requestAnimationFrame(render);

  return () => {
    stopped = true;
    cancelAnimationFrame(frame);
    draw.free();
    mesh.free();
    positionBuffer.free();
    colorBuffer.free();
    device.free();
  };
};
