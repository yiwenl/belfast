# Courtyard Gaussian renderer

Scope: implement the user's five requirements in `experiments/particles`; preserve `experiments/template` and unrelated changes. No particle simulation yet.

## Design

- Read standard binary little-endian 3DGS PLY by named properties. Decode log scale, sigmoid opacity, normalized WXYZ quaternion and SH DC color. Reject malformed/unsupported input clearly. Skip opacity below the renderer's 1/255 cutoff.
- Convert PLY coordinates with `(x, -y, -z)` to the existing Y-up, right-handed camera. Default initial eye at PLY origin, forward along original +Z, with an orbit focus one unit ahead; optional user response may select a distribution center instead.
- Upload immutable Gaussian attributes once to two textures: RGBA32Float position + covariance normalization, RGBA32Uint containing six normalized half-float covariance entries and RGBA8 color/opacity. GPU vertices project the covariance to screen ellipses and draw premultiplied Gaussian alpha back-to-front.
- Sort exact float depth keys in Rust using radix sort inside a Web Worker, keeping the main thread responsive. Transfer only u32 indices after initialization. Frustum culling includes a guard band for delayed camera updates. Only one sort is in flight.
- Compare against the last accepted sort camera; request another sort after 0.5 degrees rotation, 0.02 units translation, or projection/viewport changes. Always render with current camera uniforms.
- Retain mouse controls, FPS and Kinetic API; add loading progress and compact count/sort diagnostics.

## Steps and checks

1. PLY parser and sort policy: fixture tests for property order, truncation, covariance, color, invalid data, sorting, culling and accumulated camera movement.
2. Texture renderer and WGSL: shader validation plus native GPU rendering test with overlapping colored ellipses.
3. Wasm/worker/Vite wiring and camera initialization: full build, existing TS tests, actual courtyard browser verification, mouse interaction and stationary sort count.
4. Fresh final review of changed experiment code; resolve material findings and record validation.

## Decisions / progress

- Implementation stays in the current authorized experiment directory, whose uncommitted template copy is the required starting state. No Git integration or unrelated source changes.
- Camera origin clarification requested while independent work proceeds.
- First version supports DC color only, matching this PLY (there are no higher-order SH coefficients).
- User confirmed PLY origin, looking along original +Z.
- Parser/sort fixtures: initial missing modules RED, implementation GREEN. Shader validation explicitly enables Naga's core half-unpacking capability (does not require the WGSL f16 extension).
- Actual PLY: 3,533,087 active records from 4,320,000. Browser initial view: 1,116,197 visible, first Worker sort 48 ms, about 62 FPS. No browser errors.
- Native GPU readback passed: ellipse anisotropy, front/back blending, empty corners, invalid index rejection and initial camera pose. GPU access required an approved sandbox escalation.
- Compressed HTTP download regression: RED with compressed Content-Length, fixed by letting Fetch return the decoded body when Content-Encoding is present.
- Default `dev` now builds optimized Wasm; an unoptimized multi-million-record sorter is inappropriate for interactive use.
- Final review: fresh reviewer found no Critical, Important or Minor issues in the specified courtyard flow.
- Ruling: live model replacement is outside this one-model host; document one ExperimentApp/Worker pair per model. Reusing a pair for an in-flight replacement would require generation IDs, so that is not a supported API lifecycle.
- Ruling: browser verification covers the current WebGPU-capable desktop, not low-memory devices or every browser. No universal FPS or compatibility claim is made.
- Final verification: 12 Rust tests passed, including the full courtyard load and native GPU pixel readback; 6 TypeScript tests passed; native and Wasm package Clippy passed. Production build and Worker loaded the actual scene at localhost:4174 with no console errors (first sort 50 ms, observed 77 FPS).
