import {
  assertWebGPUSupport,
  beginRenderPass,
  BindGroup,
  Buffer,
  BufferUsage,
  createSceneTexturePipelineLayout,
  Device,
  Draw,
  Geom,
  Mesh,
  OrbitalControl,
  PerspectiveCamera,
  RenderTarget,
} from "belfast";
import { loadExrEnvMap } from "./loadExrEnvMap";
import envSkyboxShader from "./shaders/env-skybox.wgsl?raw";
import tonemapShader from "./shaders/tonemap.wgsl?raw";

const SKY_RADIUS = 50;

function multiplyMat4(out: Float32Array, a: Float32Array, b: Float32Array): void {
  for (let col = 0; col < 4; col++) {
    for (let row = 0; row < 4; row++) {
      out[col * 4 + row] =
        a[row]! * b[col * 4]! +
        a[4 + row]! * b[col * 4 + 1]! +
        a[8 + row]! * b[col * 4 + 2]! +
        a[12 + row]! * b[col * 4 + 3]!;
    }
  }
}

const viewNoTranslation = new Float32Array(16);

function skyboxViewProjection(camera: PerspectiveCamera, out: Float32Array): Float32Array {
  viewNoTranslation.set(camera.getViewMatrix() as unknown as Float32Array);
  viewNoTranslation[12] = 0;
  viewNoTranslation[13] = 0;
  viewNoTranslation[14] = 0;
  multiplyMat4(out, camera.getProjectionMatrix() as unknown as Float32Array, viewNoTranslation);
  return out;
}

async function main() {
  await assertWebGPUSupport();

  const canvas = document.createElement("canvas");
  canvas.style.cssText = "display:block;width:100vw;height:100vh;";
  document.body.appendChild(canvas);

  const status = document.createElement("div");
  status.style.cssText =
    "position:fixed;left:12px;top:12px;color:#fff;font:14px/1.4 system-ui,sans-serif;pointer-events:none;";
  status.textContent = "Loading field.hdr…";
  document.body.appendChild(status);

  const device = await Device.create(canvas, { hdr: true, alpha: false });

  const env = await loadExrEnvMap(device, "/field.hdr");
  // const env = await loadExrEnvMap(device, "/studio.exr");
  status.remove();

  const sphere = Geom.sphere({ radius: SKY_RADIUS, segments: 64 });

  const positionBuffer = Buffer.fromData(
    device,
    sphere.positions,
    BufferUsage.vertex,
    "sky-positions",
  );
  const mesh = new Mesh(sphere.positions.length / 3).addVertexBuffer({
    buffer: positionBuffer,
    arrayStride: 12,
    attributes: [{ shaderLocation: 0, format: "float32x3", offset: 0 }],
    slot: 0,
  });
  const indexBuffer = mesh.setIndexBufferFromData(device, sphere.indices, "sky-indices");

  const sceneUniformData = new Float32Array(16);
  const sceneUniformBuffer = Buffer.create(
    device,
    Buffer.uniformSize(sceneUniformData.byteLength),
    BufferUsage.uniform,
    "sky-scene-uniforms",
  );

  const RAD = Math.PI / 180;
  const camera = new PerspectiveCamera(105 * RAD, 1, 0.1, SKY_RADIUS * 2);
  const control = new OrbitalControl(camera, {
    listenerTarget: canvas,
    center: [0, 0, 0],
    radius: 0.5,
  });

  const { pipelineLayout, bindGroupLayout } = createSceneTexturePipelineLayout(device, "EnvSky");
  const skyDraw = new Draw(device, envSkyboxShader, {
    label: "EnvSkybox",
    layout: pipelineLayout,
    vertexBuffers: mesh.getVertexLayouts(),
    targets: [{ format: "rgba16float" }],
    depthStencil: {
      format: "depth24plus",
      depthWriteEnabled: false,
      depthCompare: "always",
    },
    primitive: { topology: "triangle-list", cullMode: "front" },
  });

  const sceneBindGroup = BindGroup.create(
    device,
    bindGroupLayout,
    [
      { binding: 0, resource: sceneUniformBuffer },
      { binding: 1, resource: env.view },
      { binding: 2, resource: env.sampler },
    ],
    "env-scene-bind-group",
  );

  const toneMapDraw = new Draw(device, tonemapShader, {
    label: "ToneMap",
    primitive: { topology: "triangle-list", cullMode: "none" },
  });

  const hdrTarget = RenderTarget.create(device, {
    width: 1,
    height: 1,
    format: "rgba16float",
    withDepth: true,
    label: "HdrTarget",
  });

  let toneMapBindGroup: BindGroup | null = null;
  let lastWidth = 0;
  let lastHeight = 0;

  const updateAspect = () => {
    if (canvas.width === lastWidth && canvas.height === lastHeight) {
      return;
    }
    lastWidth = canvas.width;
    lastHeight = canvas.height;
    if (lastWidth > 0 && lastHeight > 0) {
      camera.setAspect(lastWidth / lastHeight);
    }
    hdrTarget.resize(lastWidth, lastHeight);
    toneMapBindGroup = BindGroup.create(
      device,
      toneMapDraw.getBindGroupLayout(0),
      [
        { binding: 0, resource: hdrTarget.colorView },
        { binding: 1, resource: hdrTarget.sampler },
      ],
      "tonemap-bind-group",
    );
  };

  const render = () => {
    device.resize();
    updateAspect();

    skyboxViewProjection(camera, sceneUniformData);
    sceneUniformBuffer.write(device, sceneUniformData);

    const encoder = device.gpu.createCommandEncoder();

    const hdrPass = hdrTarget.beginRenderPass(encoder, {
      clearColor: { r: 0, g: 0, b: 0, a: 1 },
    });
    skyDraw.draw(hdrPass, mesh, sceneBindGroup);
    hdrPass.end();

    const screenPass = beginRenderPass(encoder, device.getCurrentTexture().createView(), {
      clearColor: { r: 0, g: 0, b: 0, a: 1 },
    });
    toneMapDraw.draw(screenPass, 3, toneMapBindGroup!);
    screenPass.end();

    device.gpu.queue.submit([encoder.finish()]);
    requestAnimationFrame(render);
  };

  window.addEventListener("beforeunload", () => {
    control.destroy();
    positionBuffer.destroy();
    indexBuffer.destroy();
    sceneUniformBuffer.destroy();
    env.texture.destroy();
    hdrTarget.destroy();
  });

  render();
}

main().catch((error) => {
  console.error(error);
});
