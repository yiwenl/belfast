import init, {
  AxisHelper,
  BindGroup,
  Buffer,
  BufferUsage,
  Device,
  OrbitalControl,
  PerspectiveCamera,
  UniformBlock,
} from "belfast-wasm";

import "../../src/style.css";

const foundCanvas = document.querySelector("canvas");
if (!foundCanvas) {
  throw new Error("Example canvas is missing");
}
const canvas = foundCanvas;

const reportError = (error: unknown) => {
  const output = document.querySelector<HTMLPreElement>("pre") ?? document.createElement("pre");
  output.textContent = error instanceof Error ? (error.stack ?? error.message) : String(error);
  document.body.append(output);
};

async function start() {
  try {
    await init();

    const device = await Device.create(canvas);
    const cameraUniforms = UniformBlock.create({ viewProj: "mat4" }, "TemplateCamera");
    const uniformBuffer = Buffer.create(
      device,
      cameraUniforms.byteSize,
      BufferUsage.uniform,
      "TemplateCamera",
    );
    const axes = new AxisHelper(device, { length: 1000 });
    const bindGroup = BindGroup.fromBuffer(device, axes, uniformBuffer, {
      label: "TemplateCameraBindGroup",
    });
    const RAD = Math.PI / 180;
    const camera = new PerspectiveCamera(
      45 * RAD,
      canvas.width / Math.max(canvas.height, 1),
      0.1,
      100,
    );
    const control = new OrbitalControl(camera, {
      listenerTarget: canvas,
      radius: 4,
      center: [0, 0, 0],
    });
    control.setYaw(40 * RAD);
    control.setPitch(30 * RAD);

    const writeCamera = () => {
      cameraUniforms.set("viewProj", camera.getViewProjectionMatrix());
      uniformBuffer.write(device, cameraUniforms);
    };

    let animationFrame = 0;
    let stopped = false;
    let lastTime = performance.now();

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
              frame.bindTarget(null, {
                clearColor: { r: 0.1, g: 0.099, b: 0.098, a: 1 },
              });
              frame.render(axes, bindGroup);
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

    window.addEventListener("beforeunload", () => {
      stopped = true;
      cancelAnimationFrame(animationFrame);
      control.destroy();
      control.free();
      bindGroup.free();
      axes.free();
      uniformBuffer.free();
      cameraUniforms.free();
      camera.free();
      device.free();
    });
  } catch (error) {
    reportError(error);
  }
}

void start();
