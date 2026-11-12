# Mountain Fold implementation

**Goal:** Controlled ridges, visible hydraulic channels and smooth depositional foothills, with monochrome mesh/line layers in one 3D scene and SVG/PNG export.

**Architecture:** Rust/Belfast owns GPU simulation, camera and rendering. TypeScript owns Tweakpane, seeded initial ridge generation and downloads. Droplets request fixed-point erosion/deposition; a grid pass applies bounded exchanges; droplets receive only the accepted amounts. Fixed simulation steps are independent of frame time. GPU visibility produces the same segments used by the preview and SVG.

**Scope:** The design and execution are authorized by the conversation. The user explicitly requests an experiment with no automated tests. Validate by building, running in WebGPU and inspecting before/after height maps; iterate shaders if ridge structure, drainage or depositional foothills are missing. Work directly in the user's new experiment. No additional approval gate.

- [x] Register the experiment; adapt the supplied fxhash/sfc32 utility and seeded ridge height field.
- [x] Implement GPU droplet exchange and height-map debug views, including reset/pause/fixed step count.
- [x] Render synchronized mesh/line halves, diffuse light, depth visibility and shadow-dependent line density.
- [x] Add all controls in Tweakpane, canvas PNG capture on Ctrl/Cmd+S, and plotter SVG export.
- [x] Build and run; visually inspect the height map, iterate erosion if necessary, verify controls/exports and record observed limitations in README.

Files: `experiments/mountain_fold/src/{scene.rs,browser.rs,erosion.wgsl,terrain.wgsl,lines.wgsl}`, `web/{main.ts,terrain.ts,hash.ts,export.ts}`, stylesheet/index and workspace manifests. Reuse template orbit handling and Belfast's existing buffer/compute interfaces. No new test files or test runs.

Review focus: concurrent erosion cannot over-credit droplets; terminal sediment is deposited before respawn; deterministic batches are frame-rate independent; split viewport depth and projection agree; screenshots capture a rendered frame; SVG contains clipped geometry rather than masking alone.

## Execution notes

- User-requested no-test workflow honored: builds and manual Chrome/WebGPU inspection only.
- Initial deposition was grainy. Added conservative flux/gather relaxation of deposited soil only; the underlying ridge substrate is not smoothed.
- Read-only reviewer found triangle/bilinear depth mismatch, stale camera controls and async filename/seed snapshot issues; all three were fixed.
- Final default run preserves ridges, cuts downhill channels and gains height around the foothills. PNG and SVG export succeeded. Same-seed rerun had identical exported height/soil channels on the inspected browser/GPU.
- Follow-up: increased mesh/height field to 512 × 512 and replaced the sharp crest with a controllable slope-continuous profile. Manual same-seed comparison at rounding 0 and 0.12 confirmed reduced crest teeth.
- Added 4× MSAA to both display halves while keeping the single-sample visibility pass used by SVG.
- Matched the requested `erosion4` style with a near-black background, white light-driven lines, a dark grayscale spotlight material and height contours, with Tweakpane controls. Final build and manual PNG/SVG checks passed; no test suites were run.
- Follow-up: replace the original split view with two vertically separated terrain layers sharing one camera and depth-tested color pass. Add Tweakpane layer spacing and a height-map toggle that defaults to off; update line visibility and SVG projection to the full viewport. Verify the composition, orbiting, zero spacing, debug toggle and exports manually.
- Camera follow-up: default spacing 6.5 and distance 30; add an orthographic toggle, perspective FOV and orthographic half-height bounds. Reuse the orbit pose for both Belfast camera types, with aspect-correct bounds, projection-aware line facing and scroll/pan behavior. Manually check switching, lens controls, shared-layer movement and exports; no automated tests.
- Orthographic inspection exposed false self-occlusion from comparing subpixel line points with a single depth pixel. Sample the four surrounding depth pixels conservatively for both preview and SVG; export the current bound directly from camera state rather than the periodically refreshed UI value.
- Final camera/layer build passed. Manual Chrome checks covered projection switching, FOV, bounds, wheel zoom, orbit and reset; PNG export succeeded in both modes and orthographic SVG produced 1,677 paths with current projection metadata. Final console check was clear. No automated tests were added or run.
- Capture follow-up: use the explicitly selected live tab's exported settings as defaults: orthographic on, bounds ±9, orbit 43°, elevation 28°, layer spacing 8; all other parameters already match. Update startup and camera reset together. Remove title/help/debug/status/error overlays; keep diagnostics inside Tweakpane and create the pane before GPU initialization so failures remain visible. Build, inspect the resulting view and commit as requested; no automated tests.
