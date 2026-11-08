# Mountain Fold

Belfast / Rust / WebGPU experiment: a seeded ridge skeleton, hydraulic erosion with visible depositional foothills, and a monochrome split view. Following `erosion4`, the preview uses a near-black background, a dark gray mesh with spotlight shading and faint height contours on the left, and white terrain-following lines on the right. Both use the same terrain, camera and light direction.

## Run

```sh
pnpm --dir experiments/mountain_fold dev
```

Open <http://localhost:5184>. Rust/WGSL edits rebuild Wasm and reload the page. TypeScript/CSS use Vite hot reload. Build a static site with:

```sh
pnpm --dir experiments/mountain_fold build
```

Output is `dist/`. WebGPU needs a supported browser on localhost or HTTPS.

## Controls

All controls are in Tweakpane. Drag either view to orbit, Shift + drag to pan, and scroll to zoom. Camera controls follow mouse movement.

- **Mountain range:** seed/fxhash, height, ridge width, ridge rounding, branch count, direction and surface noise. **Ridge rounding** controls the rounded crest as a fraction of ridge width (default 0.12); zero restores the sharp profile. The crest keeps its intended peak height and joins the original flank with a continuous slope. Changes regenerate the original terrain and restart erosion.
- **Erosion:** pause, fixed stopping step, steps per frame, single batch, water/transport parameters, deposited-soil settling and slope. Reset after changing these parameters when comparing results from the same starting point.
- **Light & lines:** live azimuth/elevation, cast shadows, mesh spotlight angle/falloff, contour spacing/strength, circular-line density, line exposure and preview stroke width. White lines follow the original eight-level light threshold: illuminated faces carry more lines. Set contour strength to zero to hide the mesh contours.
- **Height map:** lower-left GPU debug view. Choose current height, initial height, signed height change (white = gain, black = loss), remaining deposited soil, or cumulative erosion. Gain changes only the debug image. **Before erosion** switches both renderers to the initial terrain; pause separately to stop simulation.
- **Export:** PNG canvas capture, or line-only SVG with paper dimensions, margin and pen width in millimetres. **Ctrl/Cmd+S** captures the dark preview after a rendered frame, including the debug map if enabled. DOM controls are not part of the PNG. SVG retains black strokes without a background for plotting; its paths match the white preview lines.
- **Save height data:** JSON containing the four grid channels, seed, settings and step count, for inspecting the generated terrain.

The URL carries the seed. `?paused=1` starts before erosion; `?steps=2400` selects the stopping step. The default seed comes from the supplied `_templateAlfrid/src/hash.js`, whose base58 decoding and sfc32 generator are adapted in `web/hash.ts`. Arbitrary text seeds are also supported. A seed plus identical parameters and fixed steps reproduces the simulation on the same browser/GPU; cross-device bit-identical floating-point results are not promised.

## Simulation

The 512 × 512 height field spans 12 world units. The mesh uses every height sample (522,242 triangles per view) for finer ridge silhouettes. Height data uses grid-cell units. A seeded ridge/branch skeleton is generated once on the CPU; erosion runs entirely in WGSL with 1,024 concurrent droplets.

Each fixed step dispatches six passes:

1. Clear per-cell exchanges.
2. Move droplets and accumulate quantized erosion/deposition requests with integer atomics.
3. Apply exchanges per cell, limiting erosion to available material.
4. Credit droplets with actual accepted erosion and debit actual deposition.
5. Calculate downhill flux of loose deposited soil only.
6. Gather that flux, conserving transferred soil and leaving the ridge substrate untouched.

Evaporation reduces water rather than deleting sediment. Terminal droplets unload before respawning; the terrain boundary is closed. Brush exchanges use one-millionth grid-height units; terminal residuals below 0.00005 grid-height units are discarded. There is no noise injection after initialization. Pausing at the step limit leaves some sediment suspended in active droplets; reset clears the whole simulation.

The renderer samples the same triangles used by its single-sample depth prepass. A compute pass clips circular terrain-following lines against camera depth, surface orientation and light-dependent ink density. Both display halves use 4× MSAA, resolved to the canvas after drawing the mesh, lines and height-map overlay; PNG capture includes that antialiasing. Visibility and SVG remain independent of MSAA. The same visible segment buffer supplies both GPU ink and SVG. SVG export stages a snapshot, geometrically clips to the view, and joins only consecutive segments from the same circle. Curve sampling and depth visibility are finite-resolution approximations, not exact analytic hidden-line removal.

## Observed check

Per request, no automated tests were added or run. The initial 256 × 256 version was built and run in desktop Chrome/WebGPU. The default seed at 2,400 steps retained its main ridge structure, developed branching downhill channels, and gained height around the lower slopes. The initial pass produced grainy deposits; conservative loose-soil settling made the foothill transition smoother. PNG capture was visually inspected; the A5 SVG exported 1,356 visible paths and its XML structure was inspected. Camera/light changes were checked in both views. Resetting the same seed and running 2,400 steps again produced identical exported height/soil channels on this browser/GPU.

The 512 × 512 update was rebuilt and run to 2,400 steps in Chrome, then inspected with the camera at distance 12 and elevation 20°. Both views rendered and PNG capture succeeded without console errors. Grid-dependent erosion parameters are unchanged, so the finer simulation is not expected to reproduce the earlier 256 × 256 result.

The rounded-crest/MSAA update was compared at rounding 0 and 0.12 with the same seed and camera, including the initial terrain: the regular crest teeth were visibly reduced. The final dark version completed 2,400 steps without console errors; spotlight/contour controls responded, PNG capture produced the expected RGB (22, 22, 22) background, and SVG download succeeded. The preview uses height-field ray-marched shadows, so it does not reproduce `erosion4`'s filtered shadow map exactly.

This is an artistic height-field experiment. Overhangs, multilayer geology, open-boundary sediment export and physical sediment calibration are outside this version.
