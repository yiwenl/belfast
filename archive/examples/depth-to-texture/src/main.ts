import {
  assertWebGPUSupport,
  beginRenderPass,
  BindGroup,
  Buffer,
  BufferUsage,
  createSceneUniformPipelineLayout,
  Device,
  Draw,
  Geom,
  Mesh,
  OrbitalControl,
  PerspectiveCamera,
  UniformBlock,
} from "belfast";
import depthOnlyShaderCode from "./shaders/depth-only.wgsl?raw";
import depthPreviewShaderCode from "./shaders/depth-preview.wgsl?raw";
import cubeLitShaderCode from "./shaders/cube-lit.wgsl?raw";
import { mat4 } from "gl-matrix";

async function main() {
  await assertWebGPUSupport();

  const canvas = document.createElement("canvas");
  canvas.style.cssText = "display:block;width:100vw;height:100vh;";
  document.body.appendChild(canvas);

  const device = await Device.create(canvas);
  const cube = Geom.cube({ size: 1 });
  const vertexCount = cube.positions.length / 3;

  const positionBuffer = Buffer.fromData(
    device,
    cube.positions,
    BufferUsage.vertex,
    "depth-cube-positions",
  );
  const normalBuffer = Buffer.fromData(
    device,
    cube.normals,
    BufferUsage.vertex,
    "depth-cube-normals",
  );
  const mesh = new Mesh(vertexCount).addVertexBuffer({
    buffer: positionBuffer,
    arrayStride: 12,
    attributes: [{ shaderLocation: 0, format: "float32x3", offset: 0 }],
    slot: 0,
  });
  mesh.addVertexBuffer({
    buffer: normalBuffer,
    arrayStride: 12,
    attributes: [{ shaderLocation: 1, format: "float32x3", offset: 0 }],
    slot: 1,
  });
  const indexBuffer = mesh.setIndexBufferFromData(device, cube.indices, "depth-cube-indices");

  const sceneUniforms = UniformBlock.create({
    viewProj: "mat4x4f",
    model: "mat4x4f",
    lightDir: "vec4f",
  });
  const modelIdentity = mat4.create();
  const sceneUniformBuffer = Buffer.create(
    device,
    Buffer.uniformSize(sceneUniforms.byteSize),
    BufferUsage.uniform,
    "depth-scene-uniforms",
  );
  const previewParamsData = new Float32Array([0.1, 100, 2.0, 6.0]);
  const previewParamsBuffer = Buffer.fromData(
    device,
    previewParamsData,
    BufferUsage.uniform,
    "depth-preview-params",
  );

  const camera = new PerspectiveCamera(Math.PI / 4, 1, 0.1, 100);
  const control = new OrbitalControl(camera, {
    listenerTarget: canvas,
    center: [0, 0, 0],
    radius: 3,
  });

  const depthModule = device.gpu.createShaderModule({
    label: "DepthOnlyShader",
    code: depthOnlyShaderCode,
  });
  const depthPipeline = device.gpu.createRenderPipeline({
    label: "DepthOnlyPipeline",
    layout: "auto",
    vertex: {
      module: depthModule,
      entryPoint: "vs_main",
      buffers: mesh.getVertexLayouts(),
    },
    primitive: { topology: "triangle-list", cullMode: "back" },
    depthStencil: {
      format: "depth24plus",
      depthWriteEnabled: true,
      depthCompare: "less",
    },
  });

  const depthBindGroup = BindGroup.create(
    device,
    depthPipeline.getBindGroupLayout(0),
    sceneUniformBuffer,
    0,
    "depth-pass-bind-group",
  );

  const { pipelineLayout: scenePipelineLayout, bindGroupLayout: sceneBindGroupLayout } =
    createSceneUniformPipelineLayout(device, "DepthToTextureScene");
  const litCubeDraw = new Draw(device, cubeLitShaderCode, {
    label: "LitCube",
    layout: scenePipelineLayout,
    vertexBuffers: mesh.getVertexLayouts(),
    depthStencil: {
      format: "depth24plus",
      depthWriteEnabled: true,
      depthCompare: "less",
    },
  });
  const litCubeBindGroup = BindGroup.create(
    device,
    sceneBindGroupLayout,
    sceneUniformBuffer,
    0,
    "lit-cube-bind-group",
  );

  const previewDraw = new Draw(device, depthPreviewShaderCode, {
    label: "DepthPreview",
    primitive: { topology: "triangle-list", cullMode: "none" },
  });

  let depthTexture: GPUTexture | null = null;
  let depthTextureView: GPUTextureView | null = null;
  let previewBindGroup: BindGroup | null = null;
  let screenDepthTexture: GPUTexture | null = null;
  let lastWidth = 0;
  let lastHeight = 0;

  const recreateDepthTexture = (width: number, height: number) => {
    depthTexture?.destroy();
    depthTexture = device.gpu.createTexture({
      label: "shadow-depth-texture",
      size: [Math.max(1, width), Math.max(1, height)],
      format: "depth24plus",
      usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING,
    });
    depthTextureView = depthTexture.createView({ label: "shadow-depth-view" });
    previewBindGroup = BindGroup.create(
      device,
      previewDraw.getBindGroupLayout(0),
      [
        { binding: 0, resource: depthTextureView },
        { binding: 1, resource: previewParamsBuffer },
      ],
      "depth-preview-bind-group",
    );
  };

  const updateAspectAndTargets = () => {
    if (canvas.width === lastWidth && canvas.height === lastHeight) {
      return;
    }
    lastWidth = canvas.width;
    lastHeight = canvas.height;
    if (lastWidth > 0 && lastHeight > 0) {
      camera.setAspect(lastWidth / lastHeight);
    }
    recreateDepthTexture(lastWidth, lastHeight);
    screenDepthTexture?.destroy();
    screenDepthTexture = device.gpu.createTexture({
      label: "screen-depth-texture",
      size: [Math.max(1, lastWidth), Math.max(1, lastHeight)],
      format: "depth24plus",
      usage: GPUTextureUsage.RENDER_ATTACHMENT,
    });
  };

  const render = () => {
    device.resize();
    updateAspectAndTargets();

    sceneUniforms.set("viewProj", camera.getViewProjectionMatrix());
    sceneUniforms.set("model", modelIdentity);
    sceneUniforms.set("lightDir", [-0.6, -0.7, -0.4, 0]);
    sceneUniforms.writeToBuffer(sceneUniformBuffer, device);

    const encoder = device.gpu.createCommandEncoder();

    const depthPass = encoder.beginRenderPass({
      colorAttachments: [],
      depthStencilAttachment: {
        view: depthTextureView!,
        depthLoadOp: "clear",
        depthClearValue: 1,
        depthStoreOp: "store",
      },
    });
    depthPass.setPipeline(depthPipeline);
    depthBindGroup.bind(depthPass, 0);
    mesh.bind(depthPass);
    depthPass.drawIndexed(mesh.getIndexCount(), 1);
    depthPass.end();

    const screenView = device.getCurrentTexture().createView();
    const screenCubePass = beginRenderPass(encoder, screenView, {
      clearColor: { r: 0, g: 0, b: 0, a: 1 },
      depthStencilAttachment: {
        view: screenDepthTexture!.createView(),
        depthLoadOp: "clear",
        depthClearValue: 1,
        depthStoreOp: "store",
      },
    });
    litCubeDraw.draw(screenCubePass, mesh, litCubeBindGroup);
    screenCubePass.end();

    const screenPreviewPass = beginRenderPass(encoder, screenView, {
      loadOp: "load",
    });
    const previewWidth = Math.floor(canvas.width * 0.32);
    const previewHeight = Math.floor(canvas.height * 0.32);
    screenPreviewPass.setViewport(
      0,
      Math.max(0, canvas.height - previewHeight),
      previewWidth,
      previewHeight,
      0,
      1,
    );
    previewDraw.draw(screenPreviewPass, 3, previewBindGroup!);
    screenPreviewPass.end();

    device.gpu.queue.submit([encoder.finish()]);
    requestAnimationFrame(render);
  };

  window.addEventListener("beforeunload", () => {
    control.destroy();
    positionBuffer.destroy();
    normalBuffer.destroy();
    indexBuffer.destroy();
    sceneUniformBuffer.destroy();
    previewParamsBuffer.destroy();
    depthTexture?.destroy();
    screenDepthTexture?.destroy();
  });

  render();
}

main().catch((error) => {
  console.error(error);
});
