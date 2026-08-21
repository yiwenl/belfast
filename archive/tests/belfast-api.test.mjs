import assert from "node:assert/strict";
import test from "node:test";

globalThis.window = {
  requestAnimationFrame() {},
};
globalThis.GPUBufferUsage = {
  MAP_READ: 1,
  MAP_WRITE: 2,
  COPY_SRC: 4,
  COPY_DST: 8,
  INDEX: 16,
  VERTEX: 32,
  UNIFORM: 64,
  STORAGE: 128,
  INDIRECT: 256,
  QUERY_RESOLVE: 512,
};
globalThis.GPUTextureUsage = {
  COPY_SRC: 1,
  COPY_DST: 2,
  TEXTURE_BINDING: 4,
  STORAGE_BINDING: 8,
  RENDER_ATTACHMENT: 16,
};
globalThis.GPUShaderStage = {
  VERTEX: 1,
  FRAGMENT: 2,
  COMPUTE: 4,
};

const { HitTestor, UniformBlock, depthOnlyTriangles, opaqueTriangles } =
  await import("../packages/belfast/dist/belfast.js");

test("UniformBlock writes mixed f32 and u32 fields into the same packed buffer", () => {
  const uniforms = UniformBlock.create({
    time: "f32",
    dt: "f32",
    maxRadius: "f32",
    count: "u32",
  });

  uniforms.set("time", 1.5).set("dt", 0.25).set("maxRadius", 12).set("count", 123);

  assert.equal(uniforms.byteSize, 16);
  assert.equal(uniforms.floatCount, 4);
  assert.deepEqual(Array.from(uniforms.toFloat32Array().slice(0, 3)), [1.5, 0.25, 12]);
  assert.equal(new Uint32Array(uniforms.toFloat32Array().buffer)[3], 123);
});

test("UniformBlock rejects invalid u32 values", () => {
  const uniforms = UniformBlock.create({ count: "u32" });

  assert.throws(() => uniforms.set("count", -1), /expects a u32/);
  assert.throws(() => uniforms.set("count", 1.5), /expects a u32/);
  assert.throws(() => uniforms.set("count", Number.NaN), /expects a u32/);
  assert.throws(() => uniforms.set("count", [1]), /expects a number/);
});

test("opaqueTriangles returns explicit Draw pipeline state", () => {
  assert.deepEqual(
    opaqueTriangles({
      colorFormat: "rgba8unorm",
      depthFormat: "depth24plus",
      cullMode: "back",
    }),
    {
      primitive: { topology: "triangle-list", cullMode: "back" },
      depthStencil: {
        format: "depth24plus",
        depthWriteEnabled: true,
        depthCompare: "less",
      },
      targets: [{ format: "rgba8unorm" }],
    },
  );
});

test("depthOnlyTriangles returns explicit DepthDraw pipeline state", () => {
  assert.deepEqual(
    depthOnlyTriangles({
      depthFormat: "depth32float",
      cullMode: "none",
    }),
    {
      primitive: { topology: "triangle-list", cullMode: "none" },
      depthFormat: "depth32float",
      depthWriteEnabled: true,
      depthCompare: "less",
    },
  );
});

test("HitTestor maps CSS pointer coordinates into scaled canvas resolution", () => {
  const target = new EventTarget();
  target.getBoundingClientRect = () => ({
    left: 10,
    top: 20,
    width: 200,
    height: 100,
    right: 210,
    bottom: 120,
    x: 10,
    y: 20,
    toJSON() {},
  });

  let screenPos = null;
  const camera = {
    generateRay(pos) {
      screenPos = Array.from(pos);
    },
    getPosition() {
      return [0, 0, 0];
    },
  };
  const geometry = {
    positions: new Float32Array([-1, -1, -1, 1, -1, -1, 0, 1, -1]),
    indices: new Uint16Array([0, 1, 2]),
  };

  const hitTestor = new HitTestor(geometry, camera, [400, 200], { listenerTarget: target });
  const event = new Event("mousemove");
  Object.defineProperty(event, "clientX", { value: 110 });
  Object.defineProperty(event, "clientY", { value: 70 });

  target.dispatchEvent(event);
  hitTestor.disconnect();

  assert.deepEqual(screenPos, [0, 0, 0]);
});
