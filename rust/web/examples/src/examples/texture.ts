import { BindGroup, Buffer, BufferUsage, Device, Draw, Mesh, Texture } from "belfast-wasm";

import type { WebExample } from "../main";
import shaderCode from "../shaders/texture.wgsl?raw";

const textureUrl = "/scattered003.jpg";

const uvs = new Float32Array([0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0]);

async function loadTexture(device: Device): Promise<Texture> {
  const response = await fetch(textureUrl);
  if (!response.ok) {
    throw new Error(
      `Failed to load texture from ${textureUrl}: ${response.status} ${response.statusText}`,
    );
  }

  const bitmap = await createImageBitmap(await response.blob());
  try {
    return Texture.fromImageBitmap(device, bitmap, {
      label: "Scattered003",
      flipY: true,
    });
  } finally {
    bitmap.close();
  }
}

function createAspectFitMesh(
  device: Device,
  uvBuffer: Buffer,
  imageWidth: number,
  imageHeight: number,
  canvasWidth: number,
  canvasHeight: number,
): Mesh {
  const imageAspect = imageWidth / imageHeight;
  const canvasAspect = canvasWidth / canvasHeight;
  const scaleX = imageAspect > canvasAspect ? 1 : imageAspect / canvasAspect;
  const scaleY = imageAspect > canvasAspect ? canvasAspect / imageAspect : 1;
  const positions = new Float32Array([
    -scaleX,
    -scaleY,
    scaleX,
    -scaleY,
    scaleX,
    scaleY,
    -scaleX,
    -scaleY,
    scaleX,
    scaleY,
    -scaleX,
    scaleY,
  ]);
  const positionBuffer = Buffer.fromData(device, positions, BufferUsage.vertex, "TexturePositions");

  try {
    return new Mesh(6)
      .addVertexBuffer(positionBuffer, {
        arrayStride: 8,
        attributes: [{ shaderLocation: 0, format: "float32x2", offset: 0 }],
        slot: 0,
      })
      .addVertexBuffer(uvBuffer, {
        arrayStride: 8,
        attributes: [{ shaderLocation: 1, format: "float32x2", offset: 0 }],
        slot: 1,
      });
  } finally {
    positionBuffer.free();
  }
}

export const textureExample: WebExample = async (canvas, reportError) => {
  const device = await Device.create(canvas);
  let imageTexture: Texture | undefined;
  let uvBuffer: Buffer | undefined;
  let draw: Draw | undefined;
  let bindGroup: BindGroup | undefined;

  try {
    imageTexture = await loadTexture(device);
    uvBuffer = Buffer.fromData(device, uvs, BufferUsage.vertex, "TextureUvs");
    device.resize();
    const mesh = createAspectFitMesh(
      device,
      uvBuffer,
      imageTexture.width,
      imageTexture.height,
      canvas.width,
      canvas.height,
    );
    draw = new Draw(device, shaderCode, mesh, { label: "Texture" });
    bindGroup = BindGroup.fromTexture(device, draw, imageTexture, {
      label: "TextureBindGroup",
    });
  } catch (error) {
    bindGroup?.free();
    draw?.free();
    uvBuffer?.free();
    imageTexture?.free();
    device.free();
    throw error;
  }

  let lastWidth = canvas.width;
  let lastHeight = canvas.height;
  let animationFrame = 0;
  let stopped = false;

  const render = () => {
    if (stopped) {
      return;
    }

    try {
      if (device.resize()) {
        if (canvas.width !== lastWidth || canvas.height !== lastHeight) {
          const mesh = createAspectFitMesh(
            device,
            uvBuffer,
            imageTexture.width,
            imageTexture.height,
            canvas.width,
            canvas.height,
          );
          draw.setMesh(mesh);
          lastWidth = canvas.width;
          lastHeight = canvas.height;
        }

        const frame = device.beginFrame();
        if (frame) {
          frame.bindTarget(null);
          frame.render(draw, bindGroup);
          frame.submit();
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
    bindGroup.free();
    draw.free();
    uvBuffer.free();
    imageTexture.free();
    device.free();
  };
};
