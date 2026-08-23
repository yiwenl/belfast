import init, {
  AxisHelper,
  BindGroup,
  Buffer,
  BufferUsage,
  CameraHelper,
  Compute,
  DepthDraw,
  Device,
  Draw,
  Geom,
  Mesh,
  OrbitalControl,
  OrthographicCamera,
  PerspectiveCamera,
  ShadowMap,
  UniformBlock,
  wgslShadowPcf3x3,
} from "@belfast/wasm";

import "../../src/style.css";
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

async function start() {
  try {
    await init();

    const device = await Device.create(canvas);
    const geom = Geom.plane({ width: 1, height: 1 });
    const particles = spawnParticles();
    const indexFormat = geom.indices instanceof Uint32Array ? "uint32" : "uint16";

    const positionBuffer = Buffer.fromData(
      device,
      geom.positions,
      BufferUsage.vertex,
      "ParticleQuadPositions",
    );
    const instanceBuffer = Buffer.fromData(
      device,
      particles,
      BufferUsage.vertexStorage,
      "ParticleInstances",
    );
    const indexBuffer = Buffer.fromIndices(device, geom.indices, "ParticleQuadIndices");
    const mesh = new Mesh(geom.positions.length / 3)
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
      .setIndexBuffer(indexBuffer, geom.indices.length, indexFormat);

    const lightR = 6.5;
    const lightCamera = new OrthographicCamera(-lightR, lightR, -lightR, lightR, 1, 16);
    lightCamera.lookAt(new Float32Array([0, 8, 0.5]), new Float32Array([0, 0, 0]));
    const lightViewProj = lightCamera.getViewProjectionMatrix();
    const [lightRight, lightUp] = viewRightUp(lightCamera.getViewMatrix());

    const cameraUniforms = UniformBlock.create(
      {
        viewProj: "mat4",
        cameraRight: "vec4",
        cameraUp: "vec4",
        lightViewProj: "mat4",
        shadowParams: "vec4",
      },
      "ParticleCamera",
    );
    cameraUniforms.set("lightViewProj", lightViewProj);
    cameraUniforms.set("shadowParams", [0.8, SHADOW_MAP_SIZE, 0.001, 0]);
    const uniformBuffer = Buffer.create(
      device,
      cameraUniforms.byteSize,
      BufferUsage.uniform,
      "ParticleCamera",
    );

    const lightUniforms = UniformBlock.create(
      {
        viewProj: "mat4",
        cameraRight: "vec4",
        cameraUp: "vec4",
      },
      "ParticleLight",
    );
    lightUniforms.set("viewProj", lightViewProj);
    lightUniforms.set("cameraRight", lightRight);
    lightUniforms.set("cameraUp", lightUp);
    const lightBuffer = Buffer.create(
      device,
      lightUniforms.byteSize,
      BufferUsage.uniform,
      "ParticleLight",
    );
    lightBuffer.write(device, lightUniforms);

    const drawShaderCode = `${wgslShadowPcf3x3()}\n${drawShaderSource}`;
    const draw = new Draw(device, drawShaderCode, mesh, {
      label: "Particles",
      depth: true,
    });
    const shadowDraw = new DepthDraw(device, shadowShaderCode, mesh, {
      label: "ParticleShadow",
    });
    const axes = new AxisHelper(device, { length: 100, depth: true });
    const lightFrustum = new CameraHelper(device, lightViewProj, {
      color: [1, 0.85, 0.2],
      depth: true,
    });
    const shadowMap = ShadowMap.create(device, {
      size: SHADOW_MAP_SIZE,
      label: "ParticleShadowMap",
    });

    const drawBindGroup = BindGroup.fromBuffer(device, draw, uniformBuffer, {
      label: "ParticleDrawBindGroup",
    });
    const shadowBindGroup = BindGroup.fromShadowMap(device, draw, shadowMap, {
      label: "ParticleShadowBindGroup",
    });
    const lightBindGroup = BindGroup.fromBuffer(device, shadowDraw, lightBuffer, {
      label: "ParticleLightBindGroup",
    });
    const axisBindGroup = BindGroup.fromBuffer(device, axes, uniformBuffer, {
      label: "ParticleAxisBindGroup",
    });
    const frustumBindGroup = BindGroup.fromBuffer(device, lightFrustum, uniformBuffer, {
      label: "ParticleFrustumBindGroup",
    });

    let time = Math.random() * 1000;
    const simParams = UniformBlock.create(
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
    );
    simParams.set("time", time);
    simParams.set("dt", 0.016);
    simParams.set("maxRadius", MAX_RADIUS);
    simParams.set("count", PARTICLE_COUNT);
    simParams.set("noiseScale", 0.6);
    simParams.set("forceScale", 2);
    simParams.set("damping", 0.986);
    simParams.set("centerForce", 4);
    const simBuffer = Buffer.create(
      device,
      simParams.byteSize,
      BufferUsage.uniform,
      "ParticleSimParams",
    );
    simBuffer.write(device, simParams);

    const compute = new Compute(device, updateShaderCode, {
      label: "ParticleUpdate",
      layout: [
        { binding: 0, type: "uniform", minBindingSize: simParams.byteSize },
        { binding: 1, type: "storage", minBindingSize: instanceBuffer.size },
      ],
    });
    const simBindGroup = BindGroup.fromBuffers(
      device,
      compute,
      [
        { binding: 0, buffer: simBuffer },
        { binding: 1, buffer: instanceBuffer },
      ],
      { label: "ParticleSimBindGroup" },
    );

    const camera = new PerspectiveCamera(
      Math.PI / 4,
      canvas.width / Math.max(canvas.height, 1),
      0.1,
      100,
    );
    const control = new OrbitalControl(camera, {
      listenerTarget: canvas,
      radius: 20,
      center: [0, 0, 0],
    });

    const writeCamera = () => {
      const view = camera.getViewMatrix();
      const [right, up] = viewRightUp(view);
      cameraUniforms.set("viewProj", camera.getViewProjectionMatrix());
      cameraUniforms.set("cameraRight", right);
      cameraUniforms.set("cameraUp", up);
      uniformBuffer.write(device, cameraUniforms);
    };

    const workgroups = Math.ceil(PARTICLE_COUNT / WORKGROUP_SIZE);
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
        time += deltaSeconds;
        simParams.set("time", time);
        simParams.set("dt", deltaSeconds);
        simBuffer.write(device, simParams);

        if (device.resize()) {
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
              frame.render(draw, [drawBindGroup, shadowBindGroup], PARTICLE_COUNT);
              frame.render(axes, axisBindGroup);
              frame.render(lightFrustum, frustumBindGroup);
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
      frustumBindGroup.free();
      axisBindGroup.free();
      lightBindGroup.free();
      shadowBindGroup.free();
      drawBindGroup.free();
      simBindGroup.free();
      compute.free();
      lightFrustum.free();
      axes.free();
      shadowDraw.free();
      draw.free();
      mesh.free();
      shadowMap.free();
      indexBuffer.free();
      instanceBuffer.free();
      positionBuffer.free();
      simBuffer.free();
      lightBuffer.free();
      uniformBuffer.free();
      simParams.free();
      lightUniforms.free();
      cameraUniforms.free();
      lightCamera.free();
      camera.free();
      device.free();
    });
  } catch (error) {
    reportError(error);
  }
}

void start();
