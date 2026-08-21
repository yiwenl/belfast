# Rust WASM Texture And Render Target Examples Design

## Goal

Add two browser examples under `rust/web/examples` that extend the Rust Belfast
WASM facade with reusable texture sampling and multi-pass rendering:

- `texture`: JavaScript fetches and decodes an image, passes an `ImageBitmap` to
  Rust, and renders the uploaded texture without changing its aspect ratio.
- `render-to-texture`: the existing colored triangle is rendered offscreen,
  then sampled in a second pass with the left half in color and the right half
  converted to grayscale.

The work must preserve the existing JavaScript Belfast library and the current
single-pass Rust WASM example. It must add general resource and frame APIs rather
than example-specific renderer methods.

## Scope

The implementation adds the minimum browser-facing slice of these existing
Rust Belfast concepts:

- `Texture`
- `BindGroup`
- `RenderTarget`
- Rust-owned multi-pass command recording through `Frame`

The first slice supports sampled 2D float textures and filtering samplers in
bind group zero. Uniform buffers, storage textures, depth targets, multisampling,
multiple bind groups, and general render graphs remain outside this milestone.

## Architecture

### Rust-Owned Frame

JavaScript cannot safely own a raw `wgpu::CommandEncoder`, render pass, or
surface texture because their Rust lifetimes do not map to ordinary JavaScript
objects. A `Frame` class will own those resources and expose pass-sized methods:

```ts
const source = new Draw(device, sourceShader, sourceMesh, sourceDrawOptions);
const present = new Draw(device, presentShader, fullscreenMesh, presentDrawOptions);
const frame = device.beginFrame();
if (frame) {
  frame.bindTarget(renderTarget, sourcePassOptions);
  frame.render(source);
  frame.bindTarget(null, presentPassOptions);
  frame.render(present, presentBindGroup);
  frame.submit();
}
```

`Device.beginFrame()` acquires the canvas surface texture and returns
`undefined` when a frame should be skipped after timeout, occlusion, resize, or
recoverable surface loss. It returns a JavaScript error for fatal device or
validation failures. `Frame.submit()` consumes the frame, submits one command
buffer, presents the surface texture, and reports pending GPU errors.

`Frame.bindTarget(target, options)` selects the destination for the next logical
render pass. Passing a `RenderTarget` selects its offscreen texture; passing
`null`, `undefined`, or no target selects the canvas, following the WebGL
default-framebuffer convention. Calling it again closes the previous logical
pass and starts another. The only pass option in this milestone is
`clearColor: { r, g, b, a }`; omitting it uses the existing Belfast dark clear
color.

`Frame.render(draw, bindGroup?)` appends a draw command to the active logical
pass. If no target has been explicitly bound, the frame defaults to the canvas.
Multiple render calls after one `bindTarget()` belong to the same logical pass.
The frame stores an owned linear command list and materializes wgpu render
passes during `submit()`, avoiding a self-referential Rust render-pass lifetime.

Frame methods validate device ownership, draw/bind-group compatibility, and
target format before recording commands. The existing `Device.render(draw)`
remains as the single-pass convenience API and uses the same frame acquisition
and error-handling path internally.

### Draw And Mesh Ownership

The WASM `Draw` constructor consumes and owns its `Mesh`:

```ts
const draw = new Draw(device, shaderCode, mesh, options);
```

The mesh argument is not passed again to `Frame.render()` or `Device.render()`.
This differs from the native Rust and original JavaScript draw helpers, where a
pipeline can be applied to multiple meshes, but it gives the browser facade an
unambiguous owned draw command while Rust controls resource lifetimes.

`Draw.setMesh(mesh)` consumes a replacement mesh after validating that its
device and vertex-layout signature match the draw pipeline. The texture example
uses this operation to update aspect-fit positions after resize without
recreating its shader module or render pipeline.

### Texture Upload

JavaScript owns network loading and browser image decoding:

```ts
const response = await fetch("/scattered003.jpg");
const bitmap = await createImageBitmap(await response.blob());
const texture = Texture.fromImageBitmap(device, bitmap, {
  label: "Scattered003",
  flipY: true,
});
bitmap.close();
```

`Texture.fromImageBitmap()` creates a Rust-owned `wgpu::Texture`, view, and
sampler, then uses `wgpu::Queue::copy_external_image_to_texture`. The upload
keeps the browser color-space path and avoids copying a 4200 x 5940 RGBA array
through Wasm linear memory.

The core Rust `Texture` gains only the reusable construction and resource
access needed by the facade. Browser-only `ImageBitmap` types stay inside
`belfast-wasm`.

The WASM `Texture` tracks its creator device, dimensions, format, view, and
sampler. Invalid dimensions, dimensions above the device limit, unsupported
formats or filters, and cross-device use return JavaScript errors.

### Bind Groups

The JavaScript-facing class keeps the Belfast `BindGroup` vocabulary while
providing typed factories that are representable across `wasm-bindgen`:

```ts
BindGroup.fromTexture(device, draw, texture, options?)
BindGroup.fromRenderTarget(device, draw, renderTarget, options?)
```

Options default to group `0`, texture binding `0`, and sampler binding `1`.
The supported option fields are `groupIndex`, `textureBinding`,
`samplerBinding`, and `label`; unknown fields are rejected. Factories obtain
the layout from the supplied `Draw`, create the core Belfast bind group, and
retain device and draw identity. A bind group can only be used with the draw
and device that created it.

This avoids passing raw `GPUBindGroupLayout`, `GPUTextureView`, or `GPUSampler`
objects through JavaScript. A future milestone can add broader resource-entry
descriptors without changing these convenience factories.

### Shader Validation

The current WASM draw validator rejects every bound global. It will instead
accept only the supported texture slice:

```wgsl
@group(0) @binding(0) var source_texture: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
```

The validator rejects additional groups, duplicate or unexpected bindings,
storage resources, comparison samplers, unsupported texture dimensions or
sample types, and shaders that only provide one member of the required pair.
Existing vertex/fragment interface and device-limit checks remain in force.

`Draw` records whether it requires the supported texture bind group. Frame
methods reject a missing bind group for textured draws and reject a bind group
passed to a draw that has a different layout identity.

### Render Targets

The WASM `RenderTarget` wraps the existing core Rust implementation:

```ts
const target = RenderTarget.create(device, {
  width: canvas.width,
  height: canvas.height,
  label: "ColoredTriangleTarget",
});
target.resize(canvas.width, canvas.height);
```

This milestone uses one sample, no depth buffer, and the device surface format.
The target tracks its creator device. Resizing recreates its texture view, so a
bind group that samples it must be recreated after its dimensions change.

## Texture Example

The supplied source image is a 4200 x 5940, 32 MB PNG. The repository will
contain a derived 1400 x 1980 JPEG at quality 88 in
`rust/web/examples/public/scattered003.jpg`; the original desktop file remains
unchanged.

The example performs `fetch` and `createImageBitmap` in TypeScript, uploads the
bitmap through `Texture.fromImageBitmap`, and closes the bitmap immediately
after the synchronous copy call. It renders a two-triangle quad with separate
position and UV buffers.

The quad is centered and aspect-fit against the current canvas dimensions, with
the uncovered area cleared to the existing dark background. When the canvas
aspect ratio changes, the example creates a replacement position buffer and
layout-compatible mesh, then transfers that mesh into `Draw.setMesh()`; the
shader module and render pipeline remain reusable.

The example is selected with:

```text
?example=texture
```

## Render-To-Texture Example

The source pass reuses the colored triangle positions, colors, and source WGSL.
It renders to a canvas-sized `RenderTarget` using the same surface-compatible
format.

The presentation pass uses a fullscreen triangle generated from
`@builtin(vertex_index)`. Its fragment shader samples the target and applies:

- `uv.x < 0.5`: original sampled color;
- `uv.x >= 0.5`: grayscale luminance using standard perceptual RGB weights.

The split is vertical and exactly centered. On canvas resize, the render target
is resized and the presentation bind group is recreated before the next frame.

The example is selected with:

```text
?example=render-to-texture
```

## Ownership And Cleanup

Every WASM GPU resource retains stable creator-device identity. `Frame`,
`Texture`, `BindGroup`, and `RenderTarget` reject cross-device combinations
before wgpu receives them.

Each example cleanup function stops its animation loop and explicitly frees
frames/resources in dependency order. A frame that is dropped without
`submit()` does not present partial work; Rust drops the encoder and acquired
surface texture safely.

## Error Handling

- Failed HTTP fetch or image decoding is reported by the TypeScript example.
- Invalid image dimensions and external-image upload validation become
  JavaScript errors at the texture factory boundary.
- Unsupported shader resources fail synchronously in `Draw` construction.
- Missing or incompatible bind groups fail before render-pass recording.
- Surface timeout, occlusion, outdated state, and loss follow the existing
  recoverable frame-skip behavior.
- Device loss, out-of-memory, and uncaptured validation errors stop the example
  through its existing `reportError` callback.

Rust panics are not used for user-provided JavaScript data.

## Testing And Verification

Rust tests cover:

- texture dimension and device ownership validation;
- supported and unsupported WGSL texture/sampler declarations;
- bind-group device and draw compatibility;
- render-target resize and format compatibility;
- frame pass validation that can run without a browser.

The completed tree must pass:

```text
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p belfast-wasm --target wasm32-unknown-unknown
wasm-pack build crates/belfast-wasm --target web --out-dir ../../pkg/belfast-wasm
pnpm typecheck
pnpm build
```

Browser verification checks both example URLs at desktop and mobile-sized
viewports. The texture example must show the supplied image without stretching
or cropping. The render-to-texture example must have a colored left half and a
grayscale right half, verified with screenshots and representative pixel
samples, with no console or WebGPU errors after resize.

## Documentation

`rust/README.md` will list both new query-string example names and their build
commands. `rust/docs/rust-wgpu-api-parity.md` will record the implemented WASM
surface for `Texture`, `BindGroup`, `RenderTarget`, and multi-pass rendering as
partial where broader native functionality is still unavailable.
