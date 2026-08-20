import {
  BindGroup,
  Buffer,
  BufferUsage,
  Device,
  Draw,
  Mesh,
  OrbitalControl,
  PerspectiveCamera,
} from "belfast-wasm";

import type { WebExample } from "../main";
import shaderCode from "../shaders/camera-orbit.wgsl?raw";

const positions = new Float32Array([0.0, 0.7, 0.0, -0.65, -0.55, 0.0, 0.65, -0.55, 0.0]);
const colors = new Float32Array([1.0, 0.25, 0.2, 0.15, 0.9, 0.4, 0.2, 0.45, 1.0]);
const CAMERA_FLOAT_COUNT = 32;

export const cameraOrbit: WebExample = async (canvas, reportError) => {
  const device = await Device.create(canvas);
  const cameraData = new Float32Array(CAMERA_FLOAT_COUNT);
  const positionBuffer = Buffer.fromData(
    device,
    positions,
    BufferUsage.vertex,
    "CameraOrbitPositions",
  );
  const colorBuffer = Buffer.fromData(device, colors, BufferUsage.vertex, "CameraOrbitColors");
  const uniformBuffer = Buffer.create(
    device,
    cameraData.byteLength,
    BufferUsage.uniform,
    "CameraOrbitUniforms",
  );
  const mesh = new Mesh(3)
    .addVertexBuffer(positionBuffer, {
      arrayStride: 12,
      attributes: [{ shaderLocation: 0, format: "float32x3", offset: 0 }],
      slot: 0,
    })
    .addVertexBuffer(colorBuffer, {
      arrayStride: 12,
      attributes: [{ shaderLocation: 1, format: "float32x3", offset: 0 }],
      slot: 1,
    });
  const draw = new Draw(device, shaderCode, mesh, { label: "CameraOrbit" });
  const bindGroup = BindGroup.fromBuffer(device, draw, uniformBuffer, {
    label: "CameraOrbitBindGroup",
  });

  const camera = new PerspectiveCamera(
    Math.PI / 4,
    canvas.width / Math.max(canvas.height, 1),
    0.1,
    100,
  );
  const control = new OrbitalControl(camera, {
    listenerTarget: canvas,
    radius: 4,
    center: [0, 0, 0],
  });

  const hint = document.createElement("pre");
  hint.textContent = "drag to orbit · wheel to zoom · Space toggles input · A animates yaw";
  hint.style.background = "transparent";
  hint.style.color = "#c8d0e0";
  document.body.append(hint);

  let animationFrame = 0;
  let stopped = false;
  let lastTime = performance.now();
  let yawFront = true;

  const writeCamera = () => {
    cameraData.set(camera.getViewMatrix(), 0);
    cameraData.set(camera.getProjectionMatrix(), 16);
    uniformBuffer.writeData(device, cameraData);
  };

  const onKeyDown = (event: KeyboardEvent) => {
    if (event.code === "Space") {
      event.preventDefault();
      control.setEnabled(!control.enabled);
      return;
    }
    if (event.code === "KeyA") {
      event.preventDefault();
      control.setEnabled(false);
      control.setYaw(yawFront ? Math.PI / 2 : 0);
      yawFront = !yawFront;
    }
  };

  window.addEventListener("keydown", onKeyDown);
  device.resize();
  camera.setAspect(canvas.width / Math.max(canvas.height, 1));
  writeCamera();

  const render = () => {
    if (stopped) {
      return;
    }

    try {
      const now = performance.now();
      const deltaSeconds = Math.min(0.1, (now - lastTime) * 0.001);
      lastTime = now;
      control.update(deltaSeconds, camera);

      if (device.resize()) {
        camera.setAspect(canvas.width / Math.max(canvas.height, 1));
        writeCamera();
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
    window.removeEventListener("keydown", onKeyDown);
    hint.remove();
    control.destroy();
    control.free();
    bindGroup.free();
    draw.free();
    uniformBuffer.free();
    positionBuffer.free();
    colorBuffer.free();
    camera.free();
    device.free();
  };
};
