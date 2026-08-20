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

const canvas = document.querySelector<HTMLCanvasElement>("canvas");

if (!canvas) {
  throw new Error("Example canvas is missing");
}

const exampleCanvas = canvas;

const reportError = (error: unknown) => {
  const output = document.querySelector<HTMLPreElement>("pre") ?? document.createElement("pre");
  output.textContent = error instanceof Error ? (error.stack ?? error.message) : String(error);
  document.body.append(output);
};

async function start() {
  try {
    await init();

    const device = await Device.create(exampleCanvas);
    const cameraUniforms = UniformBlock.create({ viewProj: "mat4" }, "TemplateCamera");
    const uniformBuffer = Buffer.create(
      device,
      cameraUniforms.byteSize,
      BufferUsage.uniform,
      "TemplateCamera",
    );
    const axes = new AxisHelper(device, { length: 1.5 });
    const bindGroup = BindGroup.fromBuffer(device, axes, uniformBuffer, {
      label: "TemplateCameraBindGroup",
    });
    const camera = new PerspectiveCamera(
      Math.PI / 4,
      exampleCanvas.width / Math.max(exampleCanvas.height, 1),
      0.1,
      100,
    );
    const control = new OrbitalControl(camera, {
      listenerTarget: exampleCanvas,
      radius: 4,
      center: [0, 0, 0],
    });

    const writeCamera = () => {
      cameraUniforms.set("viewProj", camera.getViewProjectionMatrix());
      uniformBuffer.write(device, cameraUniforms);
    };

    let animationFrame = 0;
    let stopped = false;
    let lastTime = performance.now();

    device.resize();
    camera.setAspect(exampleCanvas.width / Math.max(exampleCanvas.height, 1));
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
          camera.setAspect(exampleCanvas.width / Math.max(exampleCanvas.height, 1));
          writeCamera();
          const frame = device.beginFrame();
          if (frame) {
            let frameConsumed = false;
            try {
              frame.bindTarget(null);
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
