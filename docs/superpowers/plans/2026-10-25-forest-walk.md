# Forest Walk Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking.

**Goal:** View the supplied colored point cloud while scrolling smoothly along the recorded camera poses.

**Architecture:** Rust owns PLY parsing, camera interpolation and instanced billboard rendering. TypeScript handles asset loading, wheel normalization and a small control panel. The existing experiment and branch are the intended working location.

**Tech Stack:** Rust, wgpu 30, glam, wasm-bindgen, TypeScript, Vite.

**Spec:** docs/superpowers/specs/2026-10-25-forest-walk-design.md

## Global Constraints

- Keep the supplied point positions and RGB; opaque unlit circular billboards.
- Scroll selects the adjacent frame and clamps to the 200-frame path.
- Preserve camera roll and the shared PLY/JSON coordinate system.
- Circle mask: length(point_coord) > 0.5 discards.
- Work on forest_walk in the checkout requested by the user.

## Review Focus

- Coordinate handedness, matrix row/column ordering, recorded camera roll.
- Truncated PLY, missing fields and non-finite coordinates.
- Wheel momentum, direction reversal and end-of-path clamping.
- Canvas resize/DPR, depth attachment sizing and shader color conversion.
- Loading failure, cancellation and teardown during asynchronous startup.

### Task 1: Load and render the recorded walk

**Files:** experiments/forest_walk/src/{point_cloud,camera_path,scene,browser,lib}.rs; src/points.wgsl; web/{wheel,input,main,style}; tests; scripts/prepare-assets.mjs; Cargo.toml; package.json; README.md; root Cargo.lock.

**Interfaces:**

- PointCloud::parse(&[u8]) -> Result<PointCloud, String>; PointCloud.points is a vector of position + RGBA8 vertices.
- CameraTrack::from_json(&str), sample(f32), len(); CameraNavigator::step(i32), advance(f32), pose(), index(), len().
- Wasm: loadPointCloud(bytes), loadCameras(json), stepCamera(direction), setPointSize(pixels), cameraIndex(), cameraCount(), resize(), frame().
- WheelStepper.push(delta, mode, viewportHeight, ctrlKey) -> -1 | 0 | 1.

- [x] Write and run failing parser, pose/interpolation, navigation and wheel tests.
- [x] Implement PLY decoding, camera-axis conversion, interpolation and wheel normalization; verify targeted tests pass.
- [x] Implement circular billboard shader, depth rendering, Wasm API and asset-loading/control UI.
- [x] Prepare the real assets and run the experiment's Rust and TypeScript suites, shader validation and production build.
- [x] Inspect the running browser, exercise scroll/size controls, and capture a preview.
- [x] Obtain a fresh code review, resolve important findings, and record verification.

Expected: all scoped tests and production build pass; real cloud is visible, scrolling advances/reverses the recorded camera, and size changes stay circular.

## Verification record

- Rust unit/integration/shader tests: 12 passed; real-asset integration separately passed (2,132,926 points, 200 poses).
- TypeScript tests: 12 passed. Type checking and release Wasm/Vite build passed.
- Browser: correct original RGB forest; scroll 1 -> 2 -> 1 and 1 -> 26; point diameter 2 -> 6 px; preview reported 60 FPS.
- Fresh read-only review found no critical/important issues. Its punctuation encoding finding was corrected and verified in the live title.
- Work remains in the requested forest_walk checkout, with no commit or remote publication.
- Local toolchain and restart script are outside tracked source; large prepared assets are Git-ignored.
