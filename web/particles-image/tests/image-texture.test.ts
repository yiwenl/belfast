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

test("palette upload uses nearest sampling so adjacent swatches do not create muddy colors", (t) => {
  let filters: unknown;
  t.mock.method(Texture, "fromImageBitmap", (_device, _bitmap, options) => {
    filters = [options.minFilter, options.magFilter];
    return {} as Texture;
  });
  createImageTextureBackend({ format: () => "Bgra8Unorm" } as Device, {} as Draw).upload(
    {} as ImageBitmap,
  );
  assert.deepEqual(filters, ["nearest", "nearest"]);
});

function fixture() {
  const released: string[] = [];
  const bindings: { texture: Texture; slot: number; group: BindGroup }[] = [];
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
    bind: (texture, slot) => {
      if (failBinding) throw new Error("Binding failed");
      const id = next;
      const group = { free: () => released.push(`binding-${id}`) } as BindGroup;
      bindings.push({ texture, slot, group });
      return group;
    },
  };
  return {
    backend,
    released,
    bindings,
    bitmap,
    failBinding: () => {
      failBinding = true;
    },
    create: () => ParticleImageTexture.create({} as Device, {} as Draw, backend),
  };
}

test("ping-pong retains the source image and replaces only the inactive slot", async () => {
  const f = fixture();
  const images = await f.create();
  assert.deepEqual(
    f.bindings.map((b) => b.slot),
    [0, 1],
  );
  const [a, b] = images.bindGroups;
  assert.equal(images.offset, 1);
  assert.equal(images.targetIndex, 0);
  await images.replace(new Blob());
  assert.equal(images.bindGroups[0], a);
  assert.notEqual(images.bindGroups[1], b);
  assert.equal(images.targetIndex, 1);
  assert.equal(images.offset, 0);
  assert.equal(images.transitioning, true);
  assert.ok(!f.released.includes("texture-1"));
  assert.ok(f.released.includes("texture-2"));
  images.advance(1);
  assert.equal(images.offset, 0.5);
  assert.equal(images.transitioning, true);
  images.advance(1);
  assert.equal(images.offset, 1);
  assert.equal(images.transitioning, false);
  const current = images.bindGroups[1];
  await images.replace(new Blob());
  assert.equal(images.bindGroups[1], current);
  assert.equal(images.targetIndex, 0);
  assert.equal(images.offset, 0);
  assert.ok(f.released.includes("texture-1"));
  assert.ok(!f.released.includes("texture-3"));
  images.advance(100);
  assert.equal(images.offset, 1);
  images.dispose();
  images.dispose();
  for (let id = 1; id <= 4; id++) {
    assert.equal(f.released.filter((r) => r === `texture-${id}`).length, 1);
    assert.equal(f.released.filter((r) => r === `binding-${id}`).length, 1);
  }
});

test("replacement is rejected during decoding or an unfinished transition", async () => {
  const f = fixture();
  const images = await f.create();
  let finish!: (bitmap: ImageBitmap) => void;
  let decodes = 0;
  f.backend.decode = () =>
    new Promise((resolve) => {
      decodes++;
      finish = resolve;
    });
  const pending = images.replace(new Blob());
  const duplicate = images.replace(new Blob());
  void duplicate.catch(() => {});
  assert.equal(decodes, 1);
  await assert.rejects(duplicate, /progress/);
  finish(f.bitmap("decoded"));
  await pending;
  await assert.rejects(images.replace(new Blob()), /progress/);
  assert.equal(f.bindings.length, 3);
  images.dispose();
});

test("failed binding retains both slots and transition state, releasing the candidate", async () => {
  const f = fixture();
  const images = await f.create();
  const original = images.bindGroups;
  f.failBinding();
  await assert.rejects(images.replace(new Blob()), /Binding failed/);
  assert.deepEqual(images.bindGroups, original);
  assert.equal(images.offset, 1);
  assert.equal(images.targetIndex, 0);
  assert.equal(images.transitioning, false);
  assert.ok(f.released.includes("texture-3"));
  assert.ok(!f.released.includes("texture-1"));
  assert.equal(f.released.filter((r) => r === "image-bitmap").length, 1);
  images.dispose();
});

test("invalid PNG retains both images and allows a later retry", async () => {
  const f = fixture();
  const images = await f.create();
  const original = images.bindGroups;
  f.backend.decode = async () => {
    throw new Error("Invalid PNG");
  };
  await assert.rejects(images.replace(new Blob()), /Invalid PNG/);
  assert.deepEqual(images.bindGroups, original);
  f.backend.decode = async () => f.bitmap("valid");
  await images.replace(new Blob());
  assert.equal(images.transitioning, true);
  images.dispose();
});

test("dispose during decoding closes the late bitmap without uploading to the freed device", async () => {
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
  assert.equal(f.bindings.length, 2);
  assert.equal(f.released.filter((r) => r === "late-bitmap").length, 1);
});

test("failure creating the second initial slot releases the first and both bitmaps", async () => {
  const f = fixture();
  const bind = f.backend.bind;
  f.backend.bind = (texture, slot) => {
    if (slot === 1) throw new Error("Second slot failed");
    return bind(texture, slot);
  };
  await assert.rejects(f.create(), /Second slot failed/);
  assert.deepEqual(f.released.filter((r) => r.startsWith("texture-")).sort(), [
    "texture-1",
    "texture-2",
  ]);
  assert.equal(f.released.filter((r) => r === "binding-1").length, 1);
  assert.equal(f.released.filter((r) => r === "white-bitmap").length, 2);
});
