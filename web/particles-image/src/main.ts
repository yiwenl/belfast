import init, {
  AxisHelper,
  BallHelper,
  BindGroup,
  Buffer,
  BufferUsage,
  CameraHelper,
  Compute,
  Device,
  Draw,
  Geom,
  Mesh,
  OrbitalControl,
  OrthographicCamera,
  PerspectiveCamera,
  Texture,
  UniformBlock,
  wgslShadowPcf3x3,
} from "@belfast/wasm";

import "../../src/style.css";
import { ParticleImageTexture } from "./image-texture";
import { ParticleImageClient } from "./api-client";
import { mountPromptPanel } from "./prompt-panel";
import { createImagePalette } from "./palette";
import workflow from "./workflow-api.json";
import "./style.css";
import drawShaderSource from "./shaders/particles.wgsl?raw";
import shadowShaderCode from "./shaders/particles_shadow_depth.wgsl?raw";
import updateShaderCode from "./shaders/particles_update.wgsl?raw";

const PARTICLE_COUNT = 1_000_000;
const SPAWN_RADIUS = 1.5;
const MAX_RADIUS = 2.5;
const PARTICLE_STRIDE = 48;
const PARTICLE_FLOATS = PARTICLE_STRIDE / 4;
const WORKGROUP_SIZE = 256;
const SHADOW_MAP_SIZE = 1024;

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

function randomRange(min: number, max: number): number {
  return min + Math.random() * (max - min);
}

function spawnParticles(): Float32Array {
  const data = new Float32Array(PARTICLE_COUNT * PARTICLE_FLOATS);
  for (let i = 0; i < PARTICLE_COUNT; i++) {
    const base = i * PARTICLE_FLOATS;
    const theta = Math.random() * Math.PI * 2;
    const phi = Math.acos(2 * Math.random() - 1);
    const r = Math.sqrt(Math.random()) * SPAWN_RADIUS;
    const sinPhi = Math.sin(phi);
    const grey = randomRange(0.8, 1);

    data[base + 0] = r * sinPhi * Math.cos(theta);
    data[base + 1] = r * sinPhi * Math.sin(theta);
    data[base + 2] = r * Math.cos(phi);
    data[base + 3] = randomRange(0.02, 0.04);

    data[base + 4] = randomRange(-0.2, 0.2);
    data[base + 5] = randomRange(-0.2, 0.2);
    data[base + 6] = randomRange(-0.2, 0.2);
    data[base + 7] = Math.random();

    data[base + 8] = grey;
    data[base + 9] = grey;
    data[base + 10] = grey;
    data[base + 11] = Math.random();
  }
  return data;
}

function viewRightUp(view: Float32Array | number[]): [number[], number[]] {
  return [
    [view[0], view[4], view[8], 0],
    [view[1], view[5], view[9], 0],
  ];
}

let stopped = false;
let setupFinished = false;
let released = false;
let animationFrame = 0;
let panel: { dispose(): void } | undefined;
let pendingImageTransition: { resolve(): void; reject(error: Error): void } | undefined;
const cleanup: (() => void)[] = [];

function own<T extends { free(): void }>(resource: T): T {
  cleanup.push(() => resource.free());
  return resource;
}

function dispose() {
  stopped = true;
  cancelAnimationFrame(animationFrame);
  panel?.dispose();
  pendingImageTransition?.reject(new DOMException("Renderer stopped", "AbortError"));
  pendingImageTransition = undefined;
  // An initial bitmap may still be decoding; keep the device alive until setup settles.
  if (setupFinished && !released) {
    released = true;
    for (const release of cleanup.reverse()) release();
    cleanup.length = 0;
  }
  window.removeEventListener("pagehide", dispose);
}
window.addEventListener("pagehide", dispose);
// BFCache can restore a page after pagehide has released its GPU resources.
function restoreFromPageCache(event: PageTransitionEvent) {
  if (event.persisted) window.location.reload();
}
window.addEventListener("pageshow", restoreFromPageCache);
import.meta.hot?.dispose(() => {
  window.removeEventListener("pageshow", restoreFromPageCache);
  dispose();
});

async function start() {
  try {
    await init();
    if (stopped) return;

    const device = own(await Device.create(canvas));
    console.info("[particles-image] surface", device.format());
    const geom = Geom.plane({ width: 1, height: 1 });
    const particles = spawnParticles();
    const indexFormat = geom.indices instanceof Uint32Array ? "uint32" : "uint16";

    const positionBuffer = own(
      Buffer.fromData(device, geom.positions, BufferUsage.vertex, "ParticleQuadPositions"),
    );
    const instanceBuffer = own(
      Buffer.fromData(device, particles, BufferUsage.vertexStorage, "ParticleInstances"),
    );
    const uvData = new Float32Array(PARTICLE_COUNT * 3);
    for (let i = 0; i < uvData.length; i++) uvData[i] = Math.random();
    const uvBuffer = own(
      Buffer.fromData(device, uvData, BufferUsage.vertex, "ParticleImageUvAndRandom"),
    );
    const indexBuffer = own(Buffer.fromIndices(device, geom.indices, "ParticleQuadIndices"));
    const mesh = own(
      new Mesh(geom.positions.length / 3)
        .addVertexBuffer(positionBuffer, {
          arrayStride: 12,
          attributes: [{ shaderLocation: 0, format: "vec3", offset: 0 }],
          slot: 0,
          stepMode: "vertex",
        })
        .addVertexBuffer(instanceBuffer, {
          arrayStride: PARTICLE_STRIDE,
          attributes: [
            { shaderLocation: 1, format: "vec4", offset: 0 },
            { shaderLocation: 2, format: "vec4", offset: 32 },
          ],
          slot: 1,
          stepMode: "instance",
        })
        .addVertexBuffer(uvBuffer, {
          arrayStride: 12,
          attributes: [{ shaderLocation: 3, format: "vec3", offset: 0 }],
          slot: 2,
          stepMode: "instance",
        })
        .setIndexBuffer(indexBuffer, geom.indices.length, indexFormat),
    );

    const lightPosition = new Float32Array([0, 8, 0.5]);
    const lightR = 6.5;
    const lightCamera = own(new OrthographicCamera(-lightR, lightR, -lightR, lightR, 1, 16));
    lightCamera.lookAt(lightPosition, new Float32Array([0, 0, 0]));
    const lightViewProj = lightCamera.getViewProjectionMatrix();
    const [lightRight, lightUp] = viewRightUp(lightCamera.getViewMatrix());

    const cameraUniforms = own(
      UniformBlock.create(
        {
          viewProj: "mat4",
          cameraRight: "vec4",
          cameraUp: "vec4",
          lightViewProj: "mat4",
          shadowParams: "vec4",
          transitionParams: "vec4",
        },
        "ParticleCamera",
      ),
    );
    cameraUniforms.set("lightViewProj", lightViewProj);
    cameraUniforms.set("shadowParams", [0.8, SHADOW_MAP_SIZE, 0.001, 0]);
    cameraUniforms.set("transitionParams", [1, 0, 0, 0]);
    const uniformBuffer = own(
      Buffer.create(device, cameraUniforms.byteSize, BufferUsage.uniform, "ParticleCamera"),
    );

    const lightUniforms = own(
      UniformBlock.create(
        {
          viewProj: "mat4",
          cameraRight: "vec4",
          cameraUp: "vec4",
        },
        "ParticleLight",
      ),
    );
    lightUniforms.set("viewProj", lightViewProj);
    lightUniforms.set("cameraRight", lightRight);
    lightUniforms.set("cameraUp", lightUp);
    const lightBuffer = own(
      Buffer.create(device, lightUniforms.byteSize, BufferUsage.uniform, "ParticleLight"),
    );
    lightBuffer.write(device, lightUniforms);

    const drawShaderCode = `${wgslShadowPcf3x3()}\n${drawShaderSource}`;
    const draw = own(
      new Draw(device, drawShaderCode, mesh, {
        label: "Particles",
        depth: true,
      }),
    );
    const shadowDraw = own(
      new Draw(device, shadowShaderCode, mesh, {
        label: "ParticleShadow",
        depthOnly: true,
      }),
    );
    const axes = own(new AxisHelper(device, { length: 100, depth: true }));
    const lightFrustum = own(
      new CameraHelper(device, lightViewProj, {
        color: [1, 0.85, 0.2],
        depth: true,
      }),
    );
    const lightBall = own(
      new BallHelper(device, {
        position: [0, 8, 0.5],
        radius: 0.4,
        color: [1, 0.85, 0.2, 1],
        depth: true,
      }),
    );
    const shadowMap = own(
      Texture.createShadowMap(device, {
        size: SHADOW_MAP_SIZE,
        label: "ParticleShadowMap",
      }),
    );

    const drawBindGroup = own(
      BindGroup.fromBuffer(device, draw, uniformBuffer, {
        label: "ParticleDrawBindGroup",
      }),
    );
    const shadowBindGroup = own(
      BindGroup.fromShadowMap(device, draw, shadowMap, {
        label: "ParticleShadowBindGroup",
      }),
    );
    const lightBindGroup = own(
      BindGroup.fromBuffer(device, shadowDraw, lightBuffer, {
        label: "ParticleLightBindGroup",
      }),
    );
    const axisBindGroup = own(
      BindGroup.fromBuffer(device, axes, uniformBuffer, {
        label: "ParticleAxisBindGroup",
      }),
    );
    const frustumBindGroup = own(
      BindGroup.fromBuffer(device, lightFrustum, uniformBuffer, {
        label: "ParticleFrustumBindGroup",
      }),
    );
    const lightBallBindGroup = own(
      BindGroup.fromBuffer(device, lightBall, uniformBuffer, {
        label: "ParticleLightBallBindGroup",
      }),
    );

    let time = Math.random() * 1000;
    const simParams = own(
      UniformBlock.create(
        {
          time: "f32",
          dt: "f32",
          maxRadius: "f32",
          count: "u32",
          noiseScale: "f32",
          forceScale: "f32",
          damping: "f32",
          centerForce: "f32",
        },
        "ParticleSimParams",
      ),
    );
    simParams.set("time", time);
    simParams.set("dt", 0.016);
    simParams.set("maxRadius", MAX_RADIUS);
    simParams.set("count", PARTICLE_COUNT);
    simParams.set("noiseScale", 0.6);
    simParams.set("forceScale", 2);
    simParams.set("damping", 0.986);
    simParams.set("centerForce", 4);
    const simBuffer = own(
      Buffer.create(device, simParams.byteSize, BufferUsage.uniform, "ParticleSimParams"),
    );
    simBuffer.write(device, simParams);

    const compute = own(
      new Compute(device, updateShaderCode, {
        label: "ParticleUpdate",
        layout: [
          { binding: 0, type: "uniform", minBindingSize: simParams.byteSize },
          { binding: 1, type: "storage", minBindingSize: instanceBuffer.size },
        ],
      }),
    );
    const simBindGroup = own(
      BindGroup.fromBuffers(
        device,
        compute,
        [
          { binding: 0, buffer: simBuffer },
          { binding: 1, buffer: instanceBuffer },
        ],
        { label: "ParticleSimBindGroup" },
      ),
    );

    const camera = own(
      new PerspectiveCamera(Math.PI / 4, canvas.width / Math.max(canvas.height, 1), 0.1, 100),
    );
    const control = own(
      new OrbitalControl(camera, {
        listenerTarget: canvas,
        radius: 20,
        center: [0, 0, 0],
      }),
    );

    cleanup.push(() => control.destroy());

    const writeCamera = () => {
      const view = camera.getViewMatrix();
      const [right, up] = viewRightUp(view);
      cameraUniforms.set("viewProj", camera.getViewProjectionMatrix());
      cameraUniforms.set("cameraRight", right);
      cameraUniforms.set("cameraUp", up);
      uniformBuffer.write(device, cameraUniforms);
    };

    const workgroups = Math.ceil(PARTICLE_COUNT / WORKGROUP_SIZE);
    let lastTime = performance.now();

    device.resize();
    camera.setAspect(canvas.width / Math.max(canvas.height, 1));
    writeCamera();

    const imageTexture = await ParticleImageTexture.create(device, draw);
    cleanup.push(() => imageTexture.dispose());
    setupFinished = true;
    if (stopped) {
      dispose();
      return;
    }

    const client = new ParticleImageClient({
      baseUrl: new URL("/comfy", window.location.origin).href,
      workflow,
    });
    const panelRoot = document.querySelector<HTMLElement>(".prompt-panel");
    if (!panelRoot) throw new Error("Prompt panel is missing");
    panel = mountPromptPanel(panelRoot, {
      generate: async (text, signal, timeoutMs) => {
        const result = await client.generate(text, { signal, timeoutMs });
        console.info(
          "[particles-image] generated",
          JSON.stringify({
            promptId: result.promptId,
            effectivePrompt: result.effectivePrompt,
            seed: result.seed,
            receivedMs: result.receivedMs,
            completedMs: result.completedMs,
          }),
        );
        return result;
      },
      applyImage: async (blob) => {
        const started = performance.now();
        const palette = await createImagePalette(blob);
        if (stopped) throw new DOMException("Renderer stopped", "AbortError");
        await imageTexture.replace(palette.image);
        if (stopped) throw new DOMException("Renderer stopped", "AbortError");
        console.info(
          "[particles-image] palette extracted and uploaded",
          JSON.stringify({
            colors: palette.colors,
            elapsedMs: performance.now() - started,
          }),
        );
        await new Promise<void>((resolve, reject) => {
          pendingImageTransition = { resolve, reject };
        });
        console.info(
          "[particles-image] transition completed",
          JSON.stringify({
            elapsedMs: performance.now() - started,
          }),
        );
        return palette.colors;
      },
    });

    const render = () => {
      if (stopped) {
        return;
      }

      try {
        const now = performance.now();
        const deltaSeconds = Math.min(0.1, (now - lastTime) * 0.001);
        lastTime = now;
        control.update(deltaSeconds, camera);
        time += deltaSeconds;
        simParams.set("time", time);
        simParams.set("dt", deltaSeconds);
        simBuffer.write(device, simParams);

        if (device.resize()) {
          imageTexture.advance(deltaSeconds);
          cameraUniforms.set("transitionParams", [
            imageTexture.offset,
            imageTexture.targetIndex,
            0,
            0,
          ]);
          camera.setAspect(canvas.width / Math.max(canvas.height, 1));
          writeCamera();
          const frame = device.beginFrame();
          if (frame) {
            let frameConsumed = false;
            try {
              frame.dispatch(compute, simBindGroup, workgroups);
              frame.bindTarget(shadowMap);
              frame.render(shadowDraw, lightBindGroup, PARTICLE_COUNT);
              frame.bindTarget(null, {
                clearColor: { r: 0.08, g: 0.076, b: 0.068, a: 1 },
                depth: true,
              });
              frame.render(
                draw,
                [drawBindGroup, shadowBindGroup, ...imageTexture.bindGroups],
                PARTICLE_COUNT,
              );
              frame.render(axes, axisBindGroup);
              frame.render(lightFrustum, frustumBindGroup);
              frame.render(lightBall, lightBallBindGroup);
              frameConsumed = true;
              frame.submit();
              if (!imageTexture.transitioning) {
                pendingImageTransition?.resolve();
                pendingImageTransition = undefined;
              }
            } finally {
              if (!frameConsumed) {
                frame.free();
              }
            }
          }
        }
      } catch (error) {
        dispose();
        reportError(error);
        return;
      }

      animationFrame = requestAnimationFrame(render);
    };

    animationFrame = requestAnimationFrame(render);
  } catch (error) {
    const wasStopped = stopped;
    setupFinished = true;
    dispose();
    if (!wasStopped) reportError(error);
  }
}

void start();
