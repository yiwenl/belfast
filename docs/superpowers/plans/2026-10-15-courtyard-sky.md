# Courtyard sky separation implementation plan

> Execute inline with superpowers:executing-plans. Preserve the template and original assets.

**Goal:** Separate the scanned sky from the courtyard so it can be replaced and removed from per-view sorting.

**Design:** Offline spatial selection produces a foreground PLY and an archived sky PLY, with unchanged vertex records and a reproducible report. A Rust full-screen background draws before foreground splats, using camera rotation only. Day/night gradients and an RGBA equirectangular texture share this renderer; only the foreground enters the worker sorter. This is background replacement, not relighting of baked daytime scene colors. Spatial selection is scene-specific and should be conservative at roofs.

**Constraints:** Keep original assets; no new package dependency; original-scene comparison remains available. Preserve camera start, input, skeleton API, motion thresholds and immutable splat textures. Panorama center faces original +Z, top is original -Y. Texture dimensions and byte count must be validated before GPU upload.

## Tasks

- [x] Analyze the active no-tree cloud and verify a measured sky boundary against roof details in multiple views. Add a standard-library split tool and byte-preservation/overwrite/invalid-data tests. Produce foreground + sky PLY and count/hash report.
- [x] Add `src/sky.rs` / `sky.wgsl`, wire into Scene and browser APIs (`setSkyPreset`, `setSkyImage`). Validate shader and GPU output, including translation invariance, rotation and invalid texture inputs.
- [x] Add day/night controls and panorama loading in `web/sky.ts`; retain original comparison. Default to foreground asset. Show sky exclusion separately from current foreground sort status.
- [x] Build release Wasm/Vite, run relevant Rust/TS/Python suites, inspect original vs separated scene and switching without additional sorts. Document limitations and actual counts. Request read-only final review.

## Review focus

- Conservative extraction must not clip ornate roof tips; retained scan-boundary haze must be disclosed if present.
- Background must remain fixed under camera translation and rotate coherently under orbit.
- Invalid/oversized/repeated panorama requests must not crash rendering or leak decoded bitmaps.
- Mode switches must not upload splat attributes or trigger a sort.
- Existing original/tree comparison must not accidentally combine replacement sky with the original sky splats.

## Execution record

Implemented inline within the existing experiment. Original and tree-removed PLY hashes stayed unchanged. Initial spatial-only extraction left obvious bright fringes, so the final selection adds color/scale cleanup and facade protection; both partitions preserve all record bytes. Final counts: 3,005,285 active foreground and 499,603 active sky (14.25% exclusion). Independent bytewise merge confirmed both partitions reconstruct all 4,281,277 source records.

Rust shader/rotation/image validation tests, native GPU readback (including sRGB targets), Python partition tests, TS tests and production build verified. Browser checks cover front/side/back, original comparison, day/night with unchanged sort count, and custom panorama. Review identified unbounded image decode; fixed with preflight PNG/JPEG/WebP dimensions, 8192×4096 source limit, 4096×2048 decode request, and one chooser decode at a time. New rejection tests run before bitmap allocation. Minor mixed sky/roof fringe and baked daylight remain documented limitations.
