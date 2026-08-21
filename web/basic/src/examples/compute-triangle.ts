import {
  BindGroup,
  Buffer,
  BufferUsage,
  Compute,
  Device,
  Draw,
  Mesh,
  UniformBlock,
} from "@belfast/wasm";

import type { WebExample } from "../main";
import computeShaderCode from "../shaders/compute-triangle.wgsl?raw";
import drawShaderCode from "../shaders/compute-triangle-draw.wgsl?raw";

const restPositions = new Float32Array([
  0.0, 0.7, 0.0, 1.0, -0.65, -0.55, 0.0, 1.0, 0.65, -0.55, 0.0, 1.0,
]);
const colors = new Float32Array([1.0, 0.35, 0.2, 0.2, 0.85, 0.45, 0.25, 0.45, 1.0]);

export const computeTriangle: WebExample = async (canvas, reportError) => {
  const device = await Device.create(canvas);
  const params = UniformBlock.create({ time: "f32" }, "ComputeTriangleParams");
  const paramsBuffer = Buffer.create(
    device,
    params.byteSize,
    BufferUsage.uniform,
    "ComputeTriangleParams",
  );
  const positionBuffer = Buffer.fromData(
    device,
    restPositions,
    BufferUsage.vertexStorage,
    "ComputeTrianglePositions",
  );
  const colorBuffer = Buffer.fromData(device, colors, BufferUsage.vertex, "ComputeTriangleColors");
  const compute = new Compute(device, computeShaderCode, {
    label: "ComputeTriangle",
    layout: [
      { binding: 0, type: "uniform", minBindingSize: params.byteSize },
      { binding: 1, type: "storage", minBindingSize: positionBuffer.size },
    ],
  });
  const computeBindGroup = BindGroup.fromBuffers(
    device,
    compute,
    [
      { binding: 0, buffer: paramsBuffer },
      { binding: 1, buffer: positionBuffer },
    ],
    { label: "ComputeTriangleBindGroup" },
  );
  const mesh = new Mesh(3)
    .addVertexBuffer(positionBuffer, {
      arrayStride: 16,
      attributes: [{ shaderLocation: 0, format: "vec4", offset: 0 }],
      slot: 0,
    })
    .addVertexBuffer(colorBuffer, {
      arrayStride: 12,
      attributes: [{ shaderLocation: 1, format: "vec3", offset: 0 }],
      slot: 1,
    });
  const draw = new Draw(device, drawShaderCode, mesh, { label: "ComputeTriangleDraw" });
  const startedAt = performance.now();

  let animationFrame = 0;
  let stopped = false;

  const render = () => {
    if (stopped) {
      return;
    }

    try {
      params.set("time", (performance.now() - startedAt) * 0.001);
      paramsBuffer.write(device, params);
      device.resize();
      const frame = device.beginFrame();
      if (frame) {
        let frameConsumed = false;
        try {
          frame.dispatch(compute, computeBindGroup, 1);
          frame.bindTarget(null);
          frame.render(draw);
          frameConsumed = true;
          frame.submit();
        } finally {
          if (!frameConsumed) {
            frame.free();
          }
        }
      }
    } catch (error) {
      stopped = true;
      reportError(error);
      return;
    }

    animationFrame = requestAnimationFrame(render);
  };

  animationFrame = requestAnimationFrame(render);

  return () => {
    stopped = true;
    cancelAnimationFrame(animationFrame);
    computeBindGroup.free();
    compute.free();
    draw.free();
    paramsBuffer.free();
    positionBuffer.free();
    colorBuffer.free();
    params.free();
    device.free();
  };
};
