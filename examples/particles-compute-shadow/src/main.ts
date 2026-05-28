import {
  assertWebGPUSupport,
  beginRenderPass,
  BindGroup,
  Buffer,
  BufferUsage,
  createBillboardDiscTriangle,
  Device,
  Draw,
  Mesh,
  OrbitalControl,
  OrthographicCamera,
  PerspectiveCamera,
  UniformBlock,
} from "belfast";
import computeShaderCode from "./shaders/particles-compute.wgsl?raw";
import drawShaderCode from "./shaders/particles-draw-shadow.wgsl?raw";
import shadowDepthShaderCode from "./shaders/particles-shadow-depth.wgsl?raw";

const PARTICLE_COUNT = 200_000;
const MAX_RADIUS = 22;
const SHADOW_MAP_SIZE = 2048;
const PARTICLE_STRIDE = 64;
const PARTICLE_FLOATS = PARTICLE_STRIDE / 4;
const PARTICLE_OFFSET = {
  positionSize: 0,
  velocitySpeed: 16,
  color: 32,
  extra: 48,
} as const;
const WORKGROUP_SIZE = 256;

/** scene (24) + lightViewProj (16) */
const DRAW_UNIFORM_FLOATS = 40;
const SHADOW_DEPTH_FORMAT: GPUTextureFormat = "depth32float";

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

    data[base + 0] = r * Math.sin(phi) * Math.cos(theta);
    data[base + 1] = r * Math.sin(phi) * Math.sin(theta);
    data[base + 2] = r * Math.cos(phi);
    data[base + 3] = randomRange(0.25, 0.2);

    data[base + 4] = randomRange(-0.2, 0.2);
    data[base + 5] = randomRange(-0.2, 0.2);
    data[base + 6] = randomRange(-0.2, 0.2);
    data[base + 7] = randomRange(0.2, 0.8);

    const hue = Math.random();
    const grey = randomRange(0.8, 1.0);
    data[base + 8] = grey;
    data[base + 9] = grey;
    data[base + 10] = grey;
    data[base + 11] = randomRange(0.5, 0.95);

    data[base + 12] = randomRange(0.0, 1.0);
    data[base + 13] = randomRange(0.0, 1.0);
    data[base + 14] = randomRange(0.0, 1.0);
    data[base + 15] = 0;
  }
  return data;
}

function setupLightCamera(): OrthographicCamera {
  const halfExtent = MAX_RADIUS * 1.5;
  const lightPos: [number, number, number] = [MAX_RADIUS * 1.4, MAX_RADIUS * 1.8, MAX_RADIUS * 1.4];
  const lightTarget: [number, number, number] = [0, 0, 0];
  const lightDistance = Math.hypot(
    lightPos[0] - lightTarget[0],
    lightPos[1] - lightTarget[1],
    lightPos[2] - lightTarget[2],
  );

  const lightCamera = new OrthographicCamera(
    -halfExtent,
    halfExtent,
    -halfExtent,
    halfExtent,
    0.1,
    lightDistance + MAX_RADIUS * 2,
  );
  lightCamera.lookAt(lightPos, lightTarget);
  return lightCamera;
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

  const drawUniformBuffer = Buffer.create(
    device,
    Buffer.uniformSize(DRAW_UNIFORM_FLOATS * 4),
    BufferUsage.uniform,
    "draw-uniforms",
  );
  const drawUniformData = new Float32Array(DRAW_UNIFORM_FLOATS);

  const lightUniformBuffer = Buffer.create(
    device,
    Buffer.uniformSize(PerspectiveCamera.uniformByteSize()),
    BufferUsage.uniform,
    "light-uniforms",
  );
  const lightUniformData = new Float32Array(PerspectiveCamera.uniformFloatCount);

  const simUniformBuffer = Buffer.create(
    device,
    Buffer.uniformSize(simUniforms.byteSize),
    BufferUsage.uniform,
    "sim-uniforms",
  );

  const lightCamera = setupLightCamera();

  const RAD = Math.PI / 180;
  const camera = new PerspectiveCamera(60 * RAD, 1, 0.1, 200);
  const control = new OrbitalControl(camera, {
    listenerTarget: canvas,
    center: [0, 0, 0],
    radius: 55,
  });

  const shadowMapTexture = device.device.createTexture({
    label: "shadow-map",
    size: [SHADOW_MAP_SIZE, SHADOW_MAP_SIZE],
    format: SHADOW_DEPTH_FORMAT,
    usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING,
  });
  const shadowMapView = shadowMapTexture.createView();
  const shadowCompareSampler = device.device.createSampler({
    label: "shadow-compare-sampler",
    compare: "less-equal",
    magFilter: "linear",
    minFilter: "linear",
    addressModeU: "clamp-to-edge",
    addressModeV: "clamp-to-edge",
  });

  const drawBindGroupLayout = device.device.createBindGroupLayout({
    label: "ParticlesDrawShadowBindGroupLayout",
    entries: [
      {
        binding: 0,
        visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT,
        buffer: { type: "uniform" },
      },
      {
        binding: 1,
        visibility: GPUShaderStage.FRAGMENT,
        texture: { sampleType: "depth" },
      },
      {
        binding: 2,
        visibility: GPUShaderStage.FRAGMENT,
        sampler: { type: "comparison" },
      },
    ],
  });

  const drawPipelineLayout = device.device.createPipelineLayout({
    label: "ParticlesDrawShadowPipelineLayout",
    bindGroupLayouts: [drawBindGroupLayout],
  });

  const shadowDepthBindGroupLayout = device.device.createBindGroupLayout({
    label: "ParticlesShadowDepthBindGroupLayout",
    entries: [
      {
        binding: 0,
        visibility: GPUShaderStage.VERTEX,
        buffer: { type: "uniform" },
      },
    ],
  });
  const shadowDepthPipeline = device.device.createRenderPipeline({
    label: "ParticlesShadowDepthPipeline",
    layout: device.device.createPipelineLayout({
      label: "ParticlesShadowDepthPipelineLayout",
      bindGroupLayouts: [shadowDepthBindGroupLayout],
    }),
    vertex: {
      module: device.device.createShaderModule({
        code: shadowDepthShaderCode,
        label: "ParticlesShadowDepthShader",
      }),
      entryPoint: "vs_main",
      buffers: mesh.getVertexLayouts(),
    },
    primitive: { topology: "triangle-list", cullMode: "none" },
    depthStencil: {
      format: SHADOW_DEPTH_FORMAT,
      depthWriteEnabled: true,
      depthCompare: "less",
    },
  });
  const shadowDepthBindGroup = BindGroup.create(
    device,
    shadowDepthBindGroupLayout,
    lightUniformBuffer,
    0,
    "shadow-depth-bind-group",
  );

  const draw = new Draw(device, drawShaderCode, {
    label: "ParticlesDrawShadow",
    layout: drawPipelineLayout,
    vertexBuffers: mesh.getVertexLayouts(),
    primitive: { topology: "triangle-list", cullMode: "none" },
    depthStencil: {
      format: "depth24plus",
      depthWriteEnabled: true,
      depthCompare: "less",
    },
    targets: [
      {
        format: device.format,
        blend: {
          color: { srcFactor: "src-alpha", dstFactor: "one-minus-src-alpha" },
          alpha: { srcFactor: "one", dstFactor: "one-minus-src-alpha" },
        },
      },
    ],
  });

  const drawBindGroup = BindGroup.create(
    device,
    drawBindGroupLayout,
    [
      { binding: 0, resource: drawUniformBuffer },
      { binding: 1, resource: shadowMapView },
      { binding: 2, resource: shadowCompareSampler },
    ],
    "draw-shadow-bind-group",
  );

  const computeBindGroupLayout = device.device.createBindGroupLayout({
    label: "ParticlesComputeBindGroupLayout",
    entries: [
      {
        binding: 0,
        visibility: GPUShaderStage.COMPUTE,
        buffer: { type: "uniform" },
      },
      {
        binding: 1,
        visibility: GPUShaderStage.COMPUTE,
        buffer: { type: "storage" },
      },
    ],
  });

  const computePipeline = device.device.createComputePipeline({
    label: "ParticlesComputePipeline",
    layout: device.device.createPipelineLayout({
      label: "ParticlesComputePipelineLayout",
      bindGroupLayouts: [computeBindGroupLayout],
    }),
    compute: {
      module: device.device.createShaderModule({
        code: computeShaderCode,
        label: "ParticlesComputeShader",
      }),
      entryPoint: "cs_main",
    },
  });

  const computeBindGroup = device.device.createBindGroup({
    label: "particles-compute-bind-group",
    layout: computeBindGroupLayout,
    entries: [
      { binding: 0, resource: { buffer: simUniformBuffer.gpu } },
      { binding: 1, resource: { buffer: particleBuffer.gpu } },
    ],
  });

  window.addEventListener("beforeunload", () => {
    control.destroy();
    cornerBuffer.destroy();
    particleBuffer.destroy();
    drawUniformBuffer.destroy();
    lightUniformBuffer.destroy();
    simUniformBuffer.destroy();
    shadowMapTexture.destroy();
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
    depthTexture = device.device.createTexture({
      label: "depth-texture",
      size: [width, height],
      format: "depth24plus",
      usage: GPUTextureUsage.RENDER_ATTACHMENT,
    });
    return depthTexture.createView();
  };

  const writeDrawUniforms = () => {
    camera.writeUniformData(drawUniformData);
    drawUniformData.set(lightCamera.getViewProjectionMatrix(), 24);
    drawUniformBuffer.write(device, drawUniformData);
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

    lightCamera.writeUniformData(lightUniformData);
    lightUniformBuffer.write(device, lightUniformData);
    writeDrawUniforms();

    const colorView = device.getCurrentTexture().createView();
    const depthView = ensureDepthTexture();
    const encoder = device.device.createCommandEncoder({ label: "particles-shadow-frame" });

    const computePass = encoder.beginComputePass({ label: "particles-sim" });
    computePass.setPipeline(computePipeline);
    computePass.setBindGroup(0, computeBindGroup);
    computePass.dispatchWorkgroups(Math.ceil(PARTICLE_COUNT / WORKGROUP_SIZE));
    computePass.end();

    const shadowPass = encoder.beginRenderPass({
      label: "shadow-map-pass",
      colorAttachments: [],
      depthStencilAttachment: {
        view: shadowMapView,
        depthLoadOp: "clear",
        depthClearValue: 1,
        depthStoreOp: "store",
      },
    });
    shadowPass.setPipeline(shadowDepthPipeline);
    shadowDepthBindGroup.bind(shadowPass, 0);
    mesh.bind(shadowPass);
    shadowPass.draw(vertexCount, PARTICLE_COUNT);
    shadowPass.end();

    const pass = beginRenderPass(encoder, colorView, {
      clearColor: { r: 0.08, g: 0.08, b: 0.1, a: 1 },
      depthStencilAttachment: {
        view: depthView,
        depthLoadOp: "clear",
        depthClearValue: 1,
        depthStoreOp: "store",
      },
    });

    draw.draw(pass, mesh, drawBindGroup, PARTICLE_COUNT);

    pass.end();
    device.device.queue.submit([encoder.finish()]);
    requestAnimationFrame(render);
  };

  render();
}

main().catch((error) => {
  console.error(error);
});
