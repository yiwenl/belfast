# Magnolia GLB placement

**Goal:** Load the supplied `magnolia-branches.glb` into the Rust-owned courtyard, at the original tree position. Keep the user's current 40° camera FOV and all unrelated changes.

**Design:** Vite fetches the GLB bytes. Rust parses embedded mesh data and PNG/JPEG textures using gltf/image, renders opaque triangles with base color and normal mapping plus simple ambient/directional light. Flatten node transforms, preserve authoring Y-up, fit total height to 2.8 scene units and ground the base at render coordinates `[0.05, -0.80, -5.98]`. Original-tree comparison omits the replacement model. The normal default and scanned-sky comparison include it.

Sky draws first without depth. The model writes Depth32Float; Gaussian splats then blend with depth testing enabled and depth writes disabled. This uses the existing per-splat center-depth approximation. Source PLYs are unchanged; mesh geometry never enters the splat sorter. Model height/position constants stay easy to edit in Scene.

- [x] Test real-asset geometry, textures, placement and malformed GLB rejection; implement embedded GLB loader in `src/model_data.rs`.
- [x] Add `src/model.rs`/`model.wgsl`, depth target and splat depth-tested pipeline; test shader and GPU mesh/splat occlusion.
- [x] Add Wasm `loadGlb`, browser loading/status, and documentation; verify release build and front/angled browser views.
- [x] Final focused read-only review; preserve source assets and unrelated edits.

Supported scope: opaque triangle meshes with positions, normals, UV0, embedded buffers/images, base-color factor/texture and normal texture; reject unsupported transparency/skins/morphs/extensions rather than silently misrendering. Basic material lighting is not full glTF PBR and does not cast a new shadow onto the baked scan.

Verification: native Rust suite passed; all five model tests including GPU readback passed; release Wasm + TypeScript + Vite build passed. Native Clippy reports only the existing `RADIANS` naming warning. Browser checks confirmed ground placement, textured branches, orbit input, day/night switching and no GPU errors. The existing splat desaturation/exposure grade is preserved; occlusion assertions account for it.

Review found a panic path in gltf utility readers for invalid accessor metadata. Added explicit component/dimension/normalization checks before reading POSITION, NORMAL, UV0 and indices. A regression test mutating the real asset's component types reproduced the panic before the fix and passes afterward.
