# Particles API simplification design

## Goal

Simplify the particles branch before merge by removing the new WebAssembly-only `DepthDraw` and `ShadowMap` façades. Reuse the existing `Draw` and `Texture` types, fix the two depth-pass correctness bugs found during review, and remove the unnecessary device argument from `CameraHelper::set_matrix`.

Backward compatibility with the branch-only `DepthDraw` and `ShadowMap` exports is intentionally not preserved.

## Public API

### Depth-only draws

`DrawOptions` gains one immediately used option:

```ts
interface DrawOptions {
  label?: string;
  primitive?: DrawPrimitiveState;
  depth?: boolean;
  depthOnly?: boolean;
}
```

`new Draw(device, shader, mesh, { depthOnly: true })` validates the depth-only shader interface and creates the pipeline with `belfast::DrawOptions::depth_only`. `depthOnly` and the canvas-oriented `depth` option are mutually exclusive. A depth-only shader may have either no resources or one group-0 uniform, so `Frame.render` derives its bind-group requirement from the existing `ShaderResourceLayout` just as it does for every other `Draw`.

The `DepthDraw` export, module, state, frame key, validation record, bind-group owner, render variants, and dedicated encode path are removed.

### Shadow textures

`Texture` gains a focused factory:

```ts
Texture.createShadowMap(device, { size?, label? }): Texture
```

It stores the resulting native `belfast::Texture` in the existing `TextureState`. `BindGroup.fromShadowMap` accepts `Texture`, verifies that its format is `Depth32Float`, and retains the existing texture state. `Frame.bindTarget` accepts a registered shadow texture and rejects ordinary color textures.

The `ShadowMap` export, module, single-field state wrapper, frame/source variants tied to that wrapper, and dedicated wrapper registration are removed. Necessary shadow-target branches remain but hold `Rc<TextureState>` rather than a parallel state type.

### Depth load operation

Native `Texture::begin_depth_pass` receives an explicit `wgpu::LoadOp<f32>`. Native particles pass `Clear(1.0)`. The WASM frame passes its already planned `Clear` or `Load` value so a compute-pass split does not discard earlier shadow draws.

No depth-pass options struct or compatibility overload is introduced.

### Camera helper updates

`CameraHelper::set_matrix` becomes `set_matrix(&self, view_proj)`. It writes through the device already owned by its positions buffer. WASM `CameraHelper.setMatrix` likewise accepts only the matrix.

## Internal data flow

The visible and shadow pipelines both enter the existing `WasmDraw` path. `ShaderResourceLayout::bind_group_count` remains the single source of truth for bind-group validation. Frame planning continues to group render commands by target and split around compute work; materialization resolves shadow target keys to `TextureState`, then encoding forwards the planned depth load operation.

This design does not add a generic render-resource abstraction, target trait, compatibility wrapper, deprecated alias, or future-facing texture-kind hierarchy.

## Error handling

- Reject `depthOnly: true` combined with `depth: true`.
- Continue rejecting color output from depth-only fragment shaders.
- Accept zero bind groups for a resource-free depth-only shader.
- Require the existing matching group-0 bind group for a uniform depth-only shader.
- Reject non-`Depth32Float` textures passed to `BindGroup.fromShadowMap` or `Frame.bindTarget` as shadow targets.

## Testing

Tests are added before implementation and observed failing for:

1. Depth-only draw option resolution and its mutually exclusive flags.
2. A resource-free depth-only draw requiring zero bind groups.
3. A shadow render, compute split, and second shadow render preserving `PlannedLoadOp::Load` through the encode-facing depth-load selection.
4. Shadow-map option parsing under `Texture` and rejection of non-depth textures.
5. The device-free `CameraHelper::set_matrix` API.

After focused tests pass, run workspace tests, clippy with warnings denied, both wasm32 checks, TypeScript typecheck/build, formatting checks on changed files, and `git diff --check`.
