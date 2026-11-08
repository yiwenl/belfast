# Mountain Fold

Belfast / Rust / WebGPU experiment: a seeded ridge skeleton, hydraulic erosion with visible depositional foothills, and terrain layers in one 3D scene. By default, the preview uses a warm off-white background with warm gray terrain-following lines floating above a dark gray mesh. The mesh retains `erosion4`'s spotlight shading and faint height contours. Both layers share the same terrain, camera and light direction.

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

Tweakpane is the only interface overlay. Drag the scene to orbit, Shift + drag to pan, and scroll to zoom. Camera controls follow mouse movement.

- **Mountain range:** seed/fxhash, height, ridge width, ridge rounding, branch count, direction and surface noise. **Ridge rounding** controls the rounded crest as a fraction of ridge width (default 0.12); zero restores the sharp profile. The crest keeps its intended peak height and joins the original flank with a continuous slope. **Noise floor** adds seeded, gently rolling relief to the low ground (default 0.18 world units, range 0–0.4); zero restores the flat initial floor. Its seven rotated noise octaves fade out over the lowest 0.6 world units of mountain height. Changes regenerate the original terrain and restart erosion.
- **Scene:** **Layer spacing** separates mesh and lines vertically in world space (default 8); zero places the lines on the mesh. Both move equally away from the scene center, so orbiting and zooming affect the pair together. **Base thickness** extends the mesh's four vertical walls down to a flat, closed bottom below terrain zero (default 0.6 world units, range 0–6); zero hides the base. It updates live without restarting erosion. **Height map** toggles the lower-left debug overlay and defaults to off. **Background** changes the canvas and PNG background live (default `#ede5db`).
- **Erosion:** pause, fixed stopping step, steps per frame, single batch, water/transport parameters, deposited-soil settling and slope. Reset after changing these parameters when comparing results from the same starting point.
- **Light & lines:** live azimuth/elevation, cast shadows, mesh spotlight angle/falloff, contour spacing/strength, circular-line density, line exposure, preview stroke width and **Line color** (default `#99928a`). The color picker updates the floating lines and PNG capture without changing the mesh material. **Hand wobble** adds smooth, seeded radial offsets to the floating lines (default 0.015 world units, range 0–0.08); zero restores regular circles. **Wobble scale** controls the distance between random offsets (default 0.6 world units, range 0.2–2): smaller values give finer waviness. Both update live without restarting erosion. Lines follow the original eight-level light threshold: illuminated faces carry more lines. Set contour strength to zero to hide the mesh contours.
- **Camera:** shared orbit/elevation and an **Orthographic** toggle (on by default). The captured starting view uses orbit about 45.6428°, elevation about 28.3639° and center (0, 0.8, 0). Perspective controls are **FOV (°)** (default 42) and **Distance** (default 30). Orthographic **Bounds ±** is the vertical half-span in world units (default 9 gives −9…9); horizontal bounds follow the canvas aspect ratio. Scrolling changes distance in perspective and bounds in orthographic mode. Switching preserves the orbit/pan pose and each mode's lens settings. **Reset camera** restores the captured pose, FOV and bounds while keeping the selected projection.
- **Height map settings:** choose current height, initial height, signed height change (white = gain, black = loss), remaining deposited soil, or cumulative erosion. Gain changes only the debug image. **Before erosion** switches both layers to the initial terrain; pause separately to stop simulation.
- **Export:** PNG canvas capture, or line-only SVG with paper dimensions, margin and pen width in millimetres. **Ctrl/Cmd+S** captures the current preview and background after a rendered frame, including the debug map if enabled. DOM controls are not part of the PNG. SVG retains black strokes without a background for plotting; its paths match the preview lines and its metadata includes the selected line and background colors.
- **Save height data:** JSON containing the four grid channels, seed, settings and step count, for inspecting the generated terrain.
- **Status:** collapsed Tweakpane monitors for render/export state, simulation steps and FPS. Rendering errors expand this folder and show their details there, including failures during WebGPU initialization.

The URL carries the seed. `?paused=1` starts before erosion; `?steps=2400` selects the stopping step. The default seed comes from the supplied `_templateAlfrid/src/hash.js`, whose base58 decoding and sfc32 generator are adapted in `web/hash.ts`. Arbitrary text seeds are also supported. A seed plus identical parameters and fixed steps reproduces the simulation on the same browser/GPU; cross-device bit-identical floating-point results are not promised.

## Simulation

The 512 × 512 height field spans 12 world units. Each terrain surface uses every height sample (522,242 triangles) for finer ridge silhouettes. Height data uses grid-cell units. A seeded ridge/branch skeleton is generated once on the CPU; erosion runs entirely in WGSL with 1,024 concurrent droplets.

The noise floor follows `erosion4`'s low-height FBM idea using the existing seeded noise lattice. It is added once to the initial height field, so erosion, mesh normals, shadows, lines, the debug map and exports all use the same relief. Unlike the old per-step noise addition, it does not keep injecting height during erosion. Changing its strength preserves the generated ridge skeleton and droplet seeds.

The mesh base adds four vertical walls sampled at every terrain boundary vertex and a flat bottom, using 4,090 additional triangles. It shares the mesh's color and visibility-depth passes. The cut faces use dark directional shading from the same light so their thickness remains readable outside the surface spotlight; the terrain material and floating lines are unchanged.

Each fixed step dispatches six passes:

1. Clear per-cell exchanges.
2. Move droplets and accumulate quantized erosion/deposition requests with integer atomics.
3. Apply exchanges per cell, limiting erosion to available material.
4. Credit droplets with actual accepted erosion and debit actual deposition.
5. Calculate downhill flux of loose deposited soil only.
6. Gather that flux, conserving transferred soil and leaving the ridge substrate untouched.

Evaporation reduces water rather than deleting sediment. Terminal droplets unload before respawning; the terrain boundary is closed. Brush exchanges use one-millionth grid-height units; terminal residuals below 0.00005 grid-height units are discarded. There is no noise injection after initialization. Pausing at the step limit leaves some sediment suspended in active droplets; reset clears the whole simulation.

The renderer uses one full-frame camera, with perspective or orthographic projection. A single-sample depth prepass draws the lower mesh and an invisible upper terrain surface, using the same triangles sampled by the lines. A compute pass clips circular terrain-following lines against this combined depth, surface orientation and light-dependent ink density. Orthographic facing uses a constant view direction. The invisible upper surface hides its own back lines but never covers the lower mesh in the color pass. The mesh and lines then share a depth-tested 4× MSAA render pass, resolved after the optional height-map overlay; PNG capture includes that antialiasing. Visibility and SVG remain independent of MSAA. The same visible segment buffer supplies both GPU ink and SVG, using the selected projection and full canvas aspect ratio. SVG export stages a snapshot, geometrically clips to the view, and joins only consecutive segments from the same circle. Curve sampling and depth visibility are finite-resolution approximations, not exact analytic hidden-line removal.

Depth visibility conservatively samples the four surrounding pixel centers. This avoids false self-occlusion on slopes in orthographic views, with roughly one pixel of allowance at occlusion boundaries.

Hand wobble uses a fixed seed from the terrain reset and periodic smooth noise along each circle. The offset stays attached to the terrain as the camera moves, joins across segment and circle boundaries, and is limited near the center to keep tiny circles from folding over. Displaced points are sampled on the height field before visibility checks; the same displaced segment buffer supplies both the preview and SVG.

## Observed check

Per request, no automated tests were added or run. The initial 256 × 256 version was built and run in desktop Chrome/WebGPU. The default seed at 2,400 steps retained its main ridge structure, developed branching downhill channels, and gained height around the lower slopes. The initial pass produced grainy deposits; conservative loose-soil settling made the foothill transition smoother. PNG capture was visually inspected; the A5 SVG exported 1,356 visible paths and its XML structure was inspected. Camera/light changes were checked in both views. Resetting the same seed and running 2,400 steps again produced identical exported height/soil channels on this browser/GPU.

The 512 × 512 update was rebuilt and run to 2,400 steps in Chrome, then inspected with the camera at distance 12 and elevation 20°. Both views rendered and PNG capture succeeded without console errors. Grid-dependent erosion parameters are unchanged, so the finer simulation is not expected to reproduce the earlier 256 × 256 result.

The rounded-crest/MSAA update was compared at rounding 0 and 0.12 with the same seed and camera, including the initial terrain: the regular crest teeth were visibly reduced. The final dark version completed 2,400 steps without console errors; spotlight/contour controls responded, PNG capture produced the expected RGB (22, 22, 22) background, and SVG download succeeded. The preview uses height-field ray-marched shadows, so it does not reproduce `erosion4`'s filtered shadow map exactly.

The shared-scene update was built and inspected in Chrome/WebGPU at 2,400 steps. Height-map visibility, zero/separated layer spacing, orbiting, perspective FOV, orthographic bounds, wheel zoom, projection switching and camera reset responded correctly. The new defaults are spacing 6.5, distance 30 and debug overlay off. Orthographic inspection exposed dashed visible lines; the four-pixel depth footprint removed those false gaps. Both projections exported PNGs, and the orthographic SVG contained 1,677 paths with matching projection/bounds metadata. No console warnings/errors were reported in the final run; no automated tests were added or run.

The current defaults were captured from the user's selected live tab through its height-data export. A fresh load and camera reset reproduced the captured view; the final page contained only the canvas and Tweakpane, with diagnostics inside its Status folder. Build and manual PNG capture passed without console warnings/errors. No automated tests were added or run.

This is an artistic height-field experiment. Overhangs, multilayer geology, open-boundary sediment export and physical sediment calibration are outside this version.
