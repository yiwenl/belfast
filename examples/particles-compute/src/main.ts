import {
  assertWebGPUSupport,
  beginRenderPass,
  BindGroup,
  Buffer,
  BufferUsage,
  Compute,
  createBillboardDiscTriangle,
  createSceneUniformPipelineLayout,
  Device,
  Draw,
  Mesh,
  OrbitalControl,
  PerspectiveCamera,
  UniformBlock,
} from "belfast";
import computeShaderCode from "./shaders/particles-compute.wgsl?raw";
import drawShaderCode from "./shaders/particles-draw.wgsl?raw";

const PARTICLE_COUNT = 500_000;
const MAX_RADIUS = 22;
/** WGSL `Particle`: 4× vec4, 64 bytes — same layout for storage + instancing */
const PARTICLE_STRIDE = 64;
const PARTICLE_FLOATS = PARTICLE_STRIDE / 4;
const PARTICLE_OFFSET = {
  positionSize: 0,
  velocitySpeed: 16,
  color: 32,
  extra: 48,
} as const;
const WORKGROUP_SIZE = 256;

const simUniforms = UniformBlock.create({
  time: "f32",
  deltaTime: "f32",
  maxRadius: "f32",
  curlScale: "f32",
  curlStrength: "f32",
  damping: "f32",
  boundaryStrength: "f32",
  particleCount: "f32",
});

function randomRange(min: number, max: number): number {
  return min + Math.random() * (max - min);
}

function buildInitialParticles(): Float32Array {
  const data = new Float32Array(PARTICLE_COUNT * PARTICLE_FLOATS);
  for (let i = 0; i < PARTICLE_COUNT; i++) {
    const base = i * PARTICLE_FLOATS;
    const theta = Math.random() * Math.PI * 2;
    const phi = Math.acos(2 * Math.random() - 1);
    const r = Math.cbrt(Math.random()) * MAX_RADIUS * 0.75;

    const x = r * Math.sin(phi) * Math.cos(theta);
    const y = r * Math.sin(phi) * Math.sin(theta);
    const z = r * Math.cos(phi);

    data[base + 0] = x;
    data[base + 1] = y;
    data[base + 2] = z;
    data[base + 3] = randomRange(0.08, 0.12);

    data[base + 4] = randomRange(-0.2, 0.2);
    data[base + 5] = randomRange(-0.2, 0.2);
    data[base + 6] = randomRange(-0.2, 0.2);
    // speed
    data[base + 7] = randomRange(0.2, 0.8);

    const hue = Math.random();
    data[base + 8] = 0.5 + 0.5 * Math.cos(hue * 6.28);
    data[base + 9] = 0.5 + 0.5 * Math.cos(hue * 6.28 + 2.09);
    data[base + 10] = 0.5 + 0.5 * Math.cos(hue * 6.28 + 4.18);
    data[base + 11] = randomRange(0.5, 0.95);

    data[base + 12] = randomRange(0.0, 1.0);
    data[base + 13] = randomRange(0.0, 1.0);
    data[base + 14] = randomRange(0.0, 1.0);
    data[base + 15] = 0;
  }
  return data;
}

async function main() {
  await assertWebGPUSupport();

  const canvas = document.createElement("canvas");
  canvas.style.cssText = "display:block;width:100vw;height:100vh;";
  document.body.appendChild(canvas);

  const device = await Device.create(canvas);
  const { corners, vertexCount } = createBillboardDiscTriangle();

  const particleBuffer = Buffer.fromData(
    device,
    buildInitialParticles(),
    BufferUsage.vertexStorage,
    "particle-buffer",
  );

  const cornerBuffer = Buffer.fromData(device, corners, BufferUsage.vertex, "disc-corners");

  const mesh = new Mesh(vertexCount)
    .addVertexBuffer({
      buffer: cornerBuffer,
      arrayStride: 16,
      attributes: [{ shaderLocation: 0, format: "float32x4", offset: 0 }],
      slot: 0,
      stepMode: "vertex",
    })
    .addVertexBuffer({
      buffer: particleBuffer,
      arrayStride: PARTICLE_STRIDE,
      attributes: [
        { shaderLocation: 1, format: "float32x4", offset: PARTICLE_OFFSET.positionSize },
        { shaderLocation: 2, format: "float32x4", offset: PARTICLE_OFFSET.color },
      ],
      slot: 1,
      stepMode: "instance",
    });

  const uniformBuffer = Buffer.create(
    device,
    Buffer.uniformSize(PerspectiveCamera.uniformByteSize()),
    BufferUsage.uniform,
    "camera-uniforms",
  );
  const cameraUniformData = new Float32Array(PerspectiveCamera.uniformFloatCount);

  const simUniformBuffer = Buffer.create(
    device,
    Buffer.uniformSize(simUniforms.byteSize),
    BufferUsage.uniform,
    "sim-uniforms",
  );

  const RAD = Math.PI / 180;
  const camera = new PerspectiveCamera(60 * RAD, 1, 0.1, 200);
  const control = new OrbitalControl(camera, {
    listenerTarget: canvas,
    center: [0, 0, 0],
    radius: 55,
  });

  const { pipelineLayout, bindGroupLayout } = createSceneUniformPipelineLayout(
    device,
    "ParticlesScene",
  );

  const compute = new Compute(device, computeShaderCode, {
    label: "ParticlesCompute",
    entryPoint: "cs_main",
  });

  const computeBindGroup = BindGroup.create(
    device,
    compute.getBindGroupLayout(0),
    [
      { binding: 0, resource: simUniformBuffer },
      { binding: 1, resource: particleBuffer },
    ],
    "particles-compute-bind-group",
  );

  const draw = new Draw(device, drawShaderCode, {
    label: "ParticlesDraw",
    layout: pipelineLayout,
    vertexBuffers: mesh.getVertexLayouts(),
    primitive: { topology: "triangle-list", cullMode: "none" },
    depthStencil: {
      format: "depth24plus",
      depthWriteEnabled: false,
      depthCompare: "less",
    },
    targets: [
      {
        format: device.format,
        blend: {
          color: { srcFactor: "src-alpha", dstFactor: "one", operation: "add" },
          alpha: { srcFactor: "one", dstFactor: "one", operation: "add" },
        },
      },
    ],
  });

  const bindGroup = BindGroup.create(device, bindGroupLayout, uniformBuffer, 0, "scene-bind-group");

  window.addEventListener("beforeunload", () => {
    control.destroy();
    cornerBuffer.destroy();
    particleBuffer.destroy();
    uniformBuffer.destroy();
    simUniformBuffer.destroy();
  });

  let depthTexture: GPUTexture | null = null;
  let lastWidth = 0;
  let lastHeight = 0;
  let lastTime = performance.now();

  const updateAspect = () => {
    if (canvas.width === lastWidth && canvas.height === lastHeight) {
      return;
    }
    lastWidth = canvas.width;
    lastHeight = canvas.height;
    if (lastWidth > 0 && lastHeight > 0) {
      camera.setAspect(lastWidth / lastHeight);
    }
  };

  const ensureDepthTexture = () => {
    const width = canvas.width;
    const height = canvas.height;
    if (depthTexture && depthTexture.width === width && depthTexture.height === height) {
      return depthTexture.createView();
    }
    depthTexture?.destroy();
    depthTexture = device.gpu.createTexture({
      label: "depth-texture",
      size: [width, height],
      format: "depth24plus",
      usage: GPUTextureUsage.RENDER_ATTACHMENT,
    });
    return depthTexture.createView();
  };

  const render = () => {
    device.resize();
    updateAspect();

    const now = performance.now();
    const deltaTime = Math.min((now - lastTime) * 0.001, 0.05);
    lastTime = now;
    const time = now * 0.001;

    simUniforms
      .set("time", time)
      .set("deltaTime", deltaTime)
      .set("maxRadius", MAX_RADIUS)
      .set("curlScale", 0.02)
      .set("curlStrength", 30.5)
      .set("damping", 0.99)
      .set("boundaryStrength", 28)
      .set("particleCount", PARTICLE_COUNT)
      .writeToBuffer(simUniformBuffer, device);

    camera.writeUniformData(cameraUniformData);
    uniformBuffer.write(device, cameraUniformData);

    const colorView = device.getCurrentTexture().createView();
    const depthView = ensureDepthTexture();
    const encoder = device.gpu.createCommandEncoder({ label: "particles-frame" });

    compute.run(
      encoder,
      computeBindGroup,
      Math.ceil(PARTICLE_COUNT / WORKGROUP_SIZE),
      "particles-sim",
    );

    const pass = beginRenderPass(encoder, colorView, {
      clearColor: { r: 0.02, g: 0.02, b: 0.04, a: 1 },
      depthStencilAttachment: {
        view: depthView,
        depthLoadOp: "clear",
        depthClearValue: 1,
        depthStoreOp: "store",
      },
    });

    draw.draw(pass, mesh, bindGroup, PARTICLE_COUNT);

    pass.end();
    device.gpu.queue.submit([encoder.finish()]);
    requestAnimationFrame(render);
  };

  render();
}

main().catch((error) => {
  console.error(error);
});
