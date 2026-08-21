import init, {
  AxisHelper,
  BindGroup,
  Buffer,
  BufferUsage,
  Device,
  Draw,
  Geom,
  Mesh,
  OrbitalControl,
  PerspectiveCamera,
  UniformBlock,
} from "@belfast/wasm";

import "../../src/style.css";
import shaderCode from "./shaders/instanced-cubes.wgsl?raw";

const GRID = 8;
const INSTANCE_COUNT = GRID * GRID * GRID;
const SPACING = 1.15;
const CUBE_SCALE = 0.55;

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

function buildInstanceData(): Float32Array {
  const data = new Float32Array(INSTANCE_COUNT * 8);
  const offset = ((GRID - 1) * SPACING) / 2;
  let index = 0;
  for (let z = 0; z < GRID; z++) {
    for (let y = 0; y < GRID; y++) {
      for (let x = 0; x < GRID; x++) {
        const base = index * 8;
        data[base + 0] = x * SPACING - offset;
        data[base + 1] = y * SPACING - offset;
        data[base + 2] = z * SPACING - offset;
        data[base + 3] = CUBE_SCALE;
        data[base + 4] = 0.3 + (x / Math.max(GRID - 1, 1)) * 0.7;
        data[base + 5] = 0.3 + (y / Math.max(GRID - 1, 1)) * 0.7;
        data[base + 6] = 0.3 + (z / Math.max(GRID - 1, 1)) * 0.7;
        data[base + 7] = 1;
        index += 1;
      }
    }
  }
  return data;
}

async function start() {
  try {
    await init();

    const device = await Device.create(canvas);
    const geom = Geom.cube({ size: 1 });
    const instanceData = buildInstanceData();
    const indexFormat = geom.indices instanceof Uint32Array ? "uint32" : "uint16";

    const positionBuffer = Buffer.fromData(
      device,
      geom.positions,
      BufferUsage.vertex,
      "InstancedCubePositions",
    );
    const normalBuffer = Buffer.fromData(
      device,
      geom.normals,
      BufferUsage.vertex,
      "InstancedCubeNormals",
    );
    const instanceBuffer = Buffer.fromData(
      device,
      instanceData,
      BufferUsage.vertex,
      "InstancedCubeInstances",
    );
    const indexBuffer = Buffer.fromIndices(device, geom.indices, "InstancedCubeIndices");
    const mesh = new Mesh(geom.positions.length / 3)
      .addVertexBuffer(positionBuffer, {
        arrayStride: 12,
        attributes: [{ shaderLocation: 0, format: "vec3", offset: 0 }],
        slot: 0,
        stepMode: "vertex",
      })
      .addVertexBuffer(normalBuffer, {
        arrayStride: 12,
        attributes: [{ shaderLocation: 1, format: "vec3", offset: 0 }],
        slot: 1,
        stepMode: "vertex",
      })
      .addVertexBuffer(instanceBuffer, {
        arrayStride: 32,
        attributes: [
          { shaderLocation: 2, format: "vec4", offset: 0 },
          { shaderLocation: 3, format: "vec4", offset: 16 },
        ],
        slot: 2,
        stepMode: "instance",
      })
      .setIndexBuffer(indexBuffer, geom.indices.length, indexFormat);

    const cameraUniforms = UniformBlock.create({ viewProj: "mat4" }, "InstancingCamera");
    const uniformBuffer = Buffer.create(
      device,
      cameraUniforms.byteSize,
      BufferUsage.uniform,
      "InstancingCamera",
    );
    const draw = new Draw(device, shaderCode, mesh, {
      label: "InstancedCubes",
      primitive: { cullMode: "back" },
      depth: true,
    });
    const axes = new AxisHelper(device, {
      length: 1000,
      depth: true,
    });
    const drawBindGroup = BindGroup.fromBuffer(device, draw, uniformBuffer, {
      label: "InstancedCubesBindGroup",
    });
    const axisBindGroup = BindGroup.fromBuffer(device, axes, uniformBuffer, {
      label: "InstancingAxisBindGroup",
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
      radius: 14,
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
                depth: true,
              });
              frame.render(axes, axisBindGroup);
              frame.render(draw, drawBindGroup, INSTANCE_COUNT);
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
      axisBindGroup.free();
      drawBindGroup.free();
      axes.free();
      draw.free();
      mesh.free();
      indexBuffer.free();
      instanceBuffer.free();
      normalBuffer.free();
      positionBuffer.free();
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
