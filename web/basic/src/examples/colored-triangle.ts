import { BindGroup, Buffer, BufferUsage, Device, Draw, Mesh, UniformBlock } from "@belfast/wasm";
import { mat4, vec3 } from "gl-matrix";

import type { WebExample } from "../main";
import shaderCode from "../shaders/colored-triangle.wgsl?raw";

const positions = new Float32Array([0.0, 0.7, 0.0, -0.65, -0.55, 0.0, 0.65, -0.55, 0.0]);

const colors = new Float32Array([1.0, 0.2, 0.15, 0.15, 0.95, 0.35, 0.2, 0.4, 1.0]);

const PITCH_LIMIT = Math.PI / 2 - 0.01;
const MIN_RADIUS = 0.5;
const MAX_RADIUS = 20;

export const coloredTriangle: WebExample = async (canvas, reportError) => {
  const device = await Device.create(canvas);
  const scene = UniformBlock.create(
    {
      viewProj: "mat4",
      time: "f32",
    },
    "ColoredTriangleScene",
  );
  const positionBuffer = Buffer.fromData(
    device,
    positions,
    BufferUsage.vertex,
    "ColoredTrianglePositions",
  );
  const colorBuffer = Buffer.fromData(device, colors, BufferUsage.vertex, "ColoredTriangleColors");
  const uniformBuffer = Buffer.create(
    device,
    scene.byteSize,
    BufferUsage.uniform,
    "ColoredTriangleScene",
  );
  const mesh = new Mesh(3)
    .addVertexBuffer(positionBuffer, {
      arrayStride: 12,
      attributes: [{ shaderLocation: 0, format: "vec3", offset: 0 }],
      slot: 0,
    })
    .addVertexBuffer(colorBuffer, {
      arrayStride: 12,
      attributes: [{ shaderLocation: 1, format: "vec3", offset: 0 }],
      slot: 1,
    });
  const draw = new Draw(device, shaderCode, mesh, { label: "ColoredTriangle" });
  const bindGroup = BindGroup.fromBuffer(device, draw, uniformBuffer, {
    label: "ColoredTriangleBindGroup",
  });

  const center = vec3.fromValues(0, 0, 0);
  const up = vec3.fromValues(0, 1, 0);
  const eye = vec3.create();
  const view = mat4.create();
  const projection = mat4.create();
  const viewProj = mat4.create();

  let yaw = 0.35;
  let pitch = 0.25;
  let radius = 2.4;
  let dragging = false;
  let lastX = 0;
  let lastY = 0;
  let animationFrame = 0;
  let stopped = false;
  const startedAt = performance.now();

  const updateProjection = () => {
    const aspect = canvas.width / Math.max(canvas.height, 1);
    mat4.perspective(projection, Math.PI / 4, aspect, 0.1, 100);
  };

  const writeScene = () => {
    const horizontal = Math.cos(pitch) * radius;
    eye[0] = Math.sin(yaw) * horizontal + center[0];
    eye[1] = Math.sin(pitch) * radius + center[1];
    eye[2] = Math.cos(yaw) * horizontal + center[2];
    mat4.lookAt(view, eye, center, up);
    mat4.multiply(viewProj, projection, view);
    scene.set("viewProj", viewProj);
    scene.set("time", (performance.now() - startedAt) * 0.001);
    uniformBuffer.write(device, scene);
  };

  const onPointerDown = (event: PointerEvent) => {
    dragging = true;
    lastX = event.clientX;
    lastY = event.clientY;
    canvas.setPointerCapture(event.pointerId);
  };

  const onPointerMove = (event: PointerEvent) => {
    if (!dragging) {
      return;
    }

    yaw -= (event.clientX - lastX) * 0.005;
    pitch = Math.min(PITCH_LIMIT, Math.max(-PITCH_LIMIT, pitch + (event.clientY - lastY) * 0.005));
    lastX = event.clientX;
    lastY = event.clientY;
  };

  const onPointerUp = (event: PointerEvent) => {
    dragging = false;
    if (canvas.hasPointerCapture(event.pointerId)) {
      canvas.releasePointerCapture(event.pointerId);
    }
  };

  const onWheel = (event: WheelEvent) => {
    event.preventDefault();
    radius = Math.min(MAX_RADIUS, Math.max(MIN_RADIUS, radius * Math.exp(event.deltaY * 0.001)));
  };

  canvas.addEventListener("pointerdown", onPointerDown);
  canvas.addEventListener("pointermove", onPointerMove);
  canvas.addEventListener("pointerup", onPointerUp);
  canvas.addEventListener("pointercancel", onPointerUp);
  canvas.addEventListener("wheel", onWheel, { passive: false });

  device.resize();
  updateProjection();
  writeScene();

  const render = () => {
    if (stopped) {
      return;
    }

    try {
      if (device.resize()) {
        updateProjection();
        writeScene();
        const frame = device.beginFrame();
        if (frame) {
          let frameConsumed = false;
          try {
            frame.bindTarget(null);
            frame.render(draw, bindGroup);
            frameConsumed = true;
            frame.submit();
          } finally {
            if (!frameConsumed) {
              frame.free();
            }
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
    canvas.removeEventListener("pointerdown", onPointerDown);
    canvas.removeEventListener("pointermove", onPointerMove);
    canvas.removeEventListener("pointerup", onPointerUp);
    canvas.removeEventListener("pointercancel", onPointerUp);
    canvas.removeEventListener("wheel", onWheel);
    bindGroup.free();
    draw.free();
    uniformBuffer.free();
    scene.free();
    positionBuffer.free();
    colorBuffer.free();
    device.free();
  };
};
