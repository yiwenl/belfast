import assert from "node:assert/strict";
import { test } from "node:test";
import { Texture, type BindGroup, type Device, type Draw } from "@belfast/wasm";
import {
  ParticleImageTexture,
  createImageTextureBackend,
  type ImageTextureBackend,
} from "../src/image-texture.ts";

test("upload preserves encoded PNG colors on an unorm canvas and decodes for an sRGB canvas", (t) => {
  const formats: string[] = [];
  t.mock.method(Texture, "fromImageBitmap", (_device, _bitmap, options) => {
    formats.push(options.format);
    return {} as Texture;
  });
  for (const surface of ["Bgra8Unorm", "Rgba8Unorm", "Bgra8UnormSrgb"]) {
    const backend = createImageTextureBackend({ format: () => surface } as Device, {} as Draw);
    backend.upload({} as ImageBitmap);
  }
  assert.deepEqual(formats, ["rgba8unorm", "rgba8unorm", "rgba8unorm-srgb"]);
});

function fixture() {
  const released: string[] = [];
  let next = 0;
  let failBinding = false;
  const bitmap = (id: string) => ({ close: () => released.push(id) }) as ImageBitmap;
  const backend: ImageTextureBackend = {
    white: async () => bitmap("white-bitmap"),
    decode: async () => bitmap("image-bitmap"),
    upload: () => {
      const id = ++next;
      return { free: () => released.push(`texture-${id}`) } as Texture;
    },
    bind: () => {
      if (failBinding) throw new Error("Binding failed");
      const id = next;
      return { free: () => released.push(`binding-${id}`) } as BindGroup;
    },
  };
  return {
    backend,
    released,
    bitmap,
    failBinding: () => {
      failBinding = true;
    },
    create: () => ParticleImageTexture.create({} as Device, {} as Draw, backend),
  };
}

test("replacement becomes active before the previous GPU resources are released", async () => {
  const f = fixture();
  const images = await f.create();
  const original = images.bindGroup;
  assert.deepEqual(f.released, ["white-bitmap"]);
  await images.replace(new Blob());
  assert.notEqual(images.bindGroup, original);
  assert.deepEqual(f.released, ["white-bitmap", "binding-1", "texture-1", "image-bitmap"]);
  images.dispose();
  images.dispose();
  assert.deepEqual(f.released.slice(-2), ["binding-2", "texture-2"]);
  assert.equal(f.released.length, 6);
});

test("failed binding releases the candidate and bitmap while retaining the old image", async () => {
  const f = fixture();
  const images = await f.create();
  const original = images.bindGroup;
  f.failBinding();
  await assert.rejects(images.replace(new Blob()), /Binding failed/);
  assert.equal(images.bindGroup, original);
  assert.deepEqual(f.released, ["white-bitmap", "texture-2", "image-bitmap"]);
  images.dispose();
});

test("invalid PNG leaves the active image available", async () => {
  const f = fixture();
  const images = await f.create();
  const original = images.bindGroup;
  f.backend.decode = async () => {
    throw new Error("Invalid PNG");
  };
  await assert.rejects(images.replace(new Blob()), /Invalid PNG/);
  assert.equal(images.bindGroup, original);
  assert.deepEqual(f.released, ["white-bitmap"]);
  images.dispose();
});

test("dispose during decoding closes the late bitmap without touching the freed device", async () => {
  const f = fixture();
  const images = await f.create();
  let finish!: (bitmap: ImageBitmap) => void;
  f.backend.decode = () =>
    new Promise((resolve) => {
      finish = resolve;
    });
  const pending = images.replace(new Blob());
  images.dispose();
  finish(f.bitmap("late-bitmap"));
  await assert.rejects(pending, /disposed/);
  assert.deepEqual(f.released, ["white-bitmap", "binding-1", "texture-1", "late-bitmap"]);
});

test("failed initial upload closes the white bitmap", async () => {
  const f = fixture();
  f.backend.upload = () => {
    throw new Error("Upload failed");
  };
  await assert.rejects(f.create(), /Upload failed/);
  assert.deepEqual(f.released, ["white-bitmap"]);
});
