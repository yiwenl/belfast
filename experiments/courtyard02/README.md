# Courtyard 02

Minimal copy of `experiments/particles`: 3D Gaussian splat rendering with worker depth sorting, frustum culling, orbital control, and flower source markers.

Run `pnpm --dir experiments/courtyard02 dev`, then open http://localhost:5184.
Drag to orbit, Shift + drag or middle-button drag to pan, and scroll to zoom.

The Tweakpane `1920 × 1080` switch defaults to enabled: the canvas drawing buffer stays exactly 1920 × 1080, independent of device pixel ratio, while CSS centers and scales the image to fit the window at 16:9. Disable it to fill the window and render at its size and device pixel ratio. Loading and rendering errors are reported in the browser console.

The local asset `public/courtyard1.ply` is copied from `/Users/yi-wenlin/Desktop/projects/courtyard02/courtyard1.ply` (1,920,000 source Gaussians). PLY files are ignored by Git; copy the asset into place on another machine before running or building.

## Initial camera

The starting view matches the active `Camera` in `court.blend`, saved on 2026-10-27 at frame 32. Its evaluated Blender position is `[-2.47359848, -3.54104233, 1.15470922]`; applying the flower-marker calibration gives scene position `[-2.25369680, 0.15622632, 1.73432976]` and forward direction `[0.53912560, 0.09760270, -0.83655083]`. Orbit starts at this pose immediately, with its pivot on the viewing ray at the flower cloud's depth.

The camera uses Blender's 35 mm lens, 36 mm sensor, AUTO sensor fit, zero lens shift, and clipping range of 0.1–100. At the default fixed 1920 × 1080 output size, horizontal FOV is 54.4322° and vertical FOV is 32.2688°. With fixed resolution disabled, resizing follows AUTO fit: horizontal FOV stays fixed in landscape, vertical FOV in portrait, so a different window aspect ratio changes the visible framing. Camera values are a snapshot, not a live Blender connection.

## Flower sources

`public/flower-emitters.json` contains 312 live marker positions from the open `court.blend` file's `WhiteFlower_Markers` collection: 215 likely (cyan) and 97 review (orange). The hidden `Flower_Emission_Points` mesh is a duplicate source and is not imported again. The live markers are used because moving a marker does not update Blender's embedded JSON or source mesh.

Each emitter's `position` is already in Belfast scene coordinates and is retained in `Scene.flowers.emitters` for a future petal simulation. The export also preserves the Blender position, quality, ID, and reference splat indices. Blender indices refer to its filtered point cloud; `ply_splat_index` refers to the original PLY.

Blender's cloud has a baked translation as well as a different up axis. The verified mapping is `[x + 0.219901681, z - 0.998482904, -y - 1.806712568]`. This was checked against 1,311 corresponding splats from the PLY, including all 312 marker reference splats; the maximum error was below 0.000003 scene units. Calibration samples and the source PLY hash are stored in the export.

The inspection balls keep Blender's radius of 0.0045 scene units and its In Front behavior: markers remain visible through geometry. The positions are heuristic flower candidates, not verified flower centers. There is no petal simulation yet.

`pnpm --dir experiments/courtyard02 build` builds WASM and the site. Per the project owner's instruction, do not run tests for this experiment unless explicitly requested.
