# Courtyard 02

Minimal copy of `experiments/particles`: 3D Gaussian splat rendering with worker depth sorting, frustum culling, orbital control, flower source markers, and instanced petals.

Run `pnpm --dir experiments/courtyard02 dev`, then open http://localhost:5184.
Drag to orbit, Shift + drag or middle-button drag to pan, and scroll to zoom.

The Tweakpane `1920 × 1080` switch defaults to enabled: the canvas drawing buffer stays exactly 1920 × 1080, independent of device pixel ratio, while CSS centers and scales the image to fit the window at 16:9. Disable it to fill the window and render at its size and device pixel ratio. Loading and rendering errors are reported in the browser console.

`Hide distance` defaults to 3.5 and ranges from 2 to 4 scene units. Splats whose centers are closer than this distance to the camera collapse to zero-area geometry in the vertex shader, before attribute loading and covariance projection. The cutoff uses Euclidean distance, takes effect without re-sorting, and avoids rasterizing those splats. Flower markers are unaffected.

`Hide flower splats` defaults to enabled. It hides white/cream splats within 0.035 scene units of any imported flower marker, while excluding green-tinted bases and foliage. Classification uses the unlit sRGB DC colors: all channels must be at least 0.30, channel spread must not exceed 28% of the largest channel, green minus red must be at most 0.025, and blue must be at least 78% of green. This is a heuristic mask, so mixed-color or unmarked flower splats can remain. The original PLY is untouched; disabling the switch restores the original splats, independently of the animated petals and debug spheres.

The browser loads flower markers before the PLY. Classification runs once before GPU upload and stores the mask in the sign of the covariance scale; the vertex shader uses its absolute magnitude for projection and collapses masked splats before attribute loading or rasterization. The 160-byte splat camera uniform reserves its final vec4 for the toggle. No extra texture, per-frame classification, or re-sorting is required. Reload the PLY after changing emitter positions to rebuild the mask.

The read-only `FPS` display averages animation-frame intervals and refreshes every half second. Sampling resets when the page visibility changes so background pauses are excluded.

## Petal model

`assets/white-petal.blend` is the editable rounded white petal, with a cupped surface, a shallow center fold, a cream body, and a pale yellow-green base. `public/white-petal.glb` is the portable model. Both contain the same 8-vertex, 8-triangle mesh, designed for petals around 0.02 scene units long and particle instancing. Two interior rows preserve the cupped shape; smooth normals soften the low-poly surface. The asset retains UVs, linear vertex colors, and a double-sided material without image textures. The shader uses simple lighting without fine vein calculations. The enlarged studio preview exposes the coarser outline; the model is intended for small on-screen petals.

The body tint is sRGB `#EEEDE0`, slightly brightened from the sampled `#E3E2D4` (the rounded channel-wise median of the brighter half of the 215 likely flower reference splats in `courtyard1.ply`). The authored base tint is sRGB `#A6B58A`, blending into cream toward the tip. The generator converts these colors to linear space before storing vertex colors in the Blender model, GLB, and runtime mesh; the renderer reads the root and tip colors from the minimum and maximum UV-Y vertices, then evaluates `smoothstep(0, 0.5, UV.y)` per fragment. The second half of the petal uses the body tint throughout, independently of the sparse mesh vertices. The portable model retains an approximation baked into vertex colors. The renderer applies its existing lighting and sRGB output conversion. This approximates the scanned flowers' palette; their baked lighting still differs from the simulated petals' dynamic lighting.

The renderer loads `public/white-petal.mesh.json`, exported from that same Blender mesh, through the existing Belfast `Mesh`/`Draw` API. The asset's local base is at the origin, its tip points along +Y, its front is +Z, and its length is 1 unit. Instances rotate this local mesh and its normals with their quaternion, then translate it to the flower source position plus jitter.

Tweakpane's `Petals` folder controls visibility, size, and `Count`: initially 50,000 petals, adjustable to 100,000 in steps of 1,000. Size remains 0.02 scene units by default (range 0.005–0.05, step 0.001). Petals are distributed evenly across all 312 imported flower positions, with independent random offsets of ±0.01 per axis, an additional uniform world-space Z offset in (-0.2, 0], and uniformly sampled unit quaternions in xyzw order. Initialization uses a fixed random seed so the layout is repeatable. The fitted shadow bounds include the entire negative-Z spawn range.

All petals share one mesh and one indexed instanced draw. Two storage buffers reserve 100,000 slots each (80 bytes per slot: position/scale, quaternion, velocity/age, source/lifetime, and spin axis/rate), but only the active count is initialized at startup. Increasing the count initializes and uploads only previously unused slots; reducing it reduces the simulation and draw count without resetting existing particles. The camera, global size, and two tint endpoints use a 112-byte uniform, and particle state is not uploaded every frame.

Petals write depth before splat rendering, so nearer splats can blend over them; the splats use center-depth testing, as in `experiments/particles`. `Flower debug` toggles the source spheres and defaults to disabled; when enabled, the spheres retain their In Front overlay behavior.

To regenerate the model, GLB, runtime mesh, and studio preview, run Blender with `--background --factory-startup --python experiments/courtyard02/tools/create_white_petal.py` from the repository root. This regenerates these asset files; it does not open or modify `court.blend`. The standalone studio image is saved as `assets/white-petal-preview.png`.

## Vortex motion

The flow uses the reusable [`belfast::filament`](../../crates/belfast/src/filament/README.md) module, adapted from [David Li's vortexspheres](https://github.com/dli/vortexspheres/tree/b2483ce3ccbd3b8f55e34c06ae2092c5f13a8a8f): regularized vortex-segment velocity, closed filament loops, self-advection, and staggered emission. The original MIT license is retained with the library in `crates/belfast/src/filament/LICENSE-vortexspheres.txt`. Filament simulation and WGSL sampling live in Belfast; petal spawning, drag, quaternion rotation, shadow boundaries, debug drawing and UI remain in this experiment.

Each frame runs two compute passes: six closed rings of twelve vertices first move through their induced flow plus an independent upward drift, then petals follow the updated flow with drag and separately controlled positive-Y acceleration. Filaments read a stable field buffer and write scratch, then copy the 72 vertices back on the GPU so particle and debug bindings remain fixed. Petals retain ping-pong storage buffers; shadow and visible draws consume the same newly written particle buffer. Normal animation uploads a 64-byte filament uniform and a 96-byte petal uniform per frame; there is no CPU particle integration or GPU readback. Changing the ring radius reseeds the filament field on the next frame, even while paused, preserving current petal positions, rotations, and active buffer. The smoothing core and velocity limits keep near-filament velocities bounded.

Tweakpane's `Motion` folder provides these controls:

| Control           | Default | Range     | Effect                                                         |
| ----------------- | ------- | --------- | -------------------------------------------------------------- |
| Animate           | On      | On/off    | Pause or resume simulation.                                    |
| Filament debug    | Off     | On/off    | Show the six live filament loops in distinct colors.           |
| Petal updraft     | 0.05    | 0–0.6     | Uniform upward acceleration on petals, in scene units/s².      |
| Filament updraft  | 0.18    | 0–0.3     | Additional upward drift of the rings, in scene units/s.        |
| Filament XZ drift | 0.06    | 0–0.2     | Maximum additional horizontal drift speed, in scene units/s.   |
| Ring radius       | 0.21    | 0.05–0.5  | Base radius of newly seeded rings, in scene units.             |
| Core radius       | 0.036   | 0.005–0.1 | Smoothing radius around each filament segment, in scene units. |
| Vortex            | 1.61    | 0–2       | Scale the induced flow strength.                               |

The smaller rings and weaker global petal updraft emphasize the local filament flow. Setting petal updraft to zero removes the global lift without stopping filament updraft. Core radius affects both compute passes immediately; ring radius also reseeds rings while paused, but petals stay frozen until resumed. Rings keep their existing ±15% respawn radius variation and can deform as they move. Independent spin axes rotate normalized quaternions, with extra tilting from the local flow. Particle ages start staggered, lifetimes are 6–10 seconds, and respawns use the original flower sources with fresh ±0.01 jitter and a newly sampled Z offset in (-0.2, 0]. Petals scale in and out at birth and death, in both the visible and shadow passes.

`Filament XZ drift` gives each ring a deterministic horizontal heading that turns slowly with simulation time. Every vertex in a ring shares that heading and a speed of 70–100% of the control value, so the added motion moves the whole ring coherently. It acts on the filaments; petals continue to follow the resulting local flow. Changes apply immediately without reseeding rings, and zero disables this additional horizontal drift. Pausing animation also pauses the drift.

`Filament debug` draws each closed loop directly from the current GPU filament buffer after simulation, including its deformation and respawns. It uses one instanced line draw, with no GPU readback or per-frame CPU geometry updates. The lines appear in front of the scene, remain visible when petals are hidden, and do not cast shadows. Disabling the toggle skips this draw.

The previously selected shadow camera remains at near/far 5/9.5 with 3× width and 6× height. Petals fade near that fixed volume's edges and respawn when they leave it, so the effect continuously fills the supported area. There are no courtyard or petal-to-petal collision calculations. Simulation steps cap at 1/30 second to avoid jumps after tab suspension; below 30 FPS the simulation advances more slowly than wall time.

## Shadows

A directional light sits at `[0, 8, 3]` and looks toward the courtyard origin. The positive Z offset avoids a view direction parallel to the Y-up vector. Petal lighting uses this same direction.

Belfast's `CameraHelper` draws the light camera's near and far planes and connecting edges as a yellow wireframe. It uses the exact shadow view-projection matrix and updates when the fitted bounds change. The lines appear in front of the scene independently of `Flower debug` and do not enter the shadow pass. Tweakpane's `Shadows` folder includes a `Camera helper` toggle (disabled by default) and `Strength` from 0 to 1 in steps of 0.01 (default 0.65).

A square preview at the bottom-left of the canvas displays the actual shadow depth texture directly on the GPU after the scene and helper overlays. Darker pixels are closer to the light; white is the far plane or empty map. The preview scales with the canvas, keeping a small inset margin. Tweakpane's `Shadows` → `Shadow map debug` toggle controls this preview (disabled by default); disabling it skips the overlay render pass. It is independent of shadow strength and the camera helper.

Before the visible scene, one additional instanced draw renders both sides of the petals into a 2048 × 2048 `Depth32Float` shadow map. It uses the same mesh, quaternion transform, size, and active count as the visible pass. Hiding petals clears their shadows too. The orthographic camera fits all flower sources plus the Z offset, jitter, and the rotated petal radius, then expands width by 3× and height by 6× around the fitted center to allow petal drift; changing size refits it. Near and far are 5 and 9.5 scene units from the light, covering above the flowers to below the courtyard floor.

Belfast's 3 × 3 PCF helper softens shadows, with default strength 0.65, comparison bias 0.00002, normal bias 0.0005, and raster depth bias. Petals receive shadows per fragment on all lighting, including ambient, direct, and transmitted light; fully occluded fragments become black at strength 1, while partially covered PCF edges remain softened. The 3DGS scene receives petal shadows at each splat center in the vertex shader, skipping shadow texture reads outside the caster footprint and applying attenuation in linear color space. This center-based approximation can lose shadow detail on large splats. The scanned courtyard retains its baked lighting and does not cast new shadows onto petals.

The local asset `public/courtyard1.ply` is copied from `/Users/yi-wenlin/Desktop/projects/courtyard02/courtyard1.ply` (1,920,000 source Gaussians). PLY files are ignored by Git; copy the asset into place on another machine before running or building.

## Initial camera

The starting view matches the active `Camera` in `court.blend`, saved on 2026-10-27 at frame 32. Its evaluated Blender position is `[-2.47359848, -3.54104233, 1.15470922]`; applying the flower-marker calibration gives scene position `[-2.25369680, 0.15622632, 1.73432976]` and forward direction `[0.53912560, 0.09760270, -0.83655083]`. Orbit starts at this pose immediately, with its pivot on the viewing ray at the flower cloud's depth.

The camera uses Blender's 35 mm lens, 36 mm sensor, AUTO sensor fit, zero lens shift, and clipping range of 0.1–100. At the default fixed 1920 × 1080 output size, horizontal FOV is 54.4322° and vertical FOV is 32.2688°. With fixed resolution disabled, resizing follows AUTO fit: horizontal FOV stays fixed in landscape, vertical FOV in portrait, so a different window aspect ratio changes the visible framing. Camera values are a snapshot, not a live Blender connection.

## Flower sources

`public/flower-emitters.json` contains 312 live marker positions from the open `court.blend` file's `WhiteFlower_Markers` collection: 215 likely (cyan) and 97 review (orange). The hidden `Flower_Emission_Points` mesh is a duplicate source and is not imported again. The live markers are used because moving a marker does not update Blender's embedded JSON or source mesh.

Each emitter's `position` is already in Belfast scene coordinates and is retained in `Scene.flowers.emitters` for spawning petals. The export also preserves the Blender position, quality, ID, and reference splat indices. Blender indices refer to its filtered point cloud; `ply_splat_index` refers to the original PLY.

Blender's cloud has a baked translation as well as a different up axis. The verified mapping is `[x + 0.219901681, z - 0.998482904, -y - 1.806712568]`. This was checked against 1,311 corresponding splats from the PLY, including all 312 marker reference splats; the maximum error was below 0.000003 scene units. Calibration samples and the source PLY hash are stored in the export.

The inspection balls keep Blender's radius of 0.0045 scene units and its In Front behavior: markers remain visible through geometry. The positions are heuristic flower candidates, not verified flower centers, and are also the spawn sources for the petal simulation.

`pnpm --dir experiments/courtyard02 build` builds WASM and the site. Per the project owner's instruction, do not run tests for this experiment unless explicitly requested.
