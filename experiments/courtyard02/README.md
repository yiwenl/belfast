# Courtyard 02

Minimal copy of `experiments/particles`: 3D Gaussian splat rendering with worker depth sorting, frustum culling, orbital control, and flower source markers.

Run `pnpm --dir experiments/courtyard02 dev`, then open http://localhost:5184.
Drag to orbit, Shift + drag or middle-button drag to pan, and scroll to zoom.

The local asset `public/courtyard1.ply` is copied from `/Users/yi-wenlin/Desktop/projects/courtyard02/courtyard1.ply` (1,920,000 source Gaussians). PLY files are ignored by Git; copy the asset into place on another machine before running or building.

## Flower sources

`public/flower-emitters.json` contains 312 live marker positions from the open `court.blend` file's `WhiteFlower_Markers` collection: 215 likely (cyan) and 97 review (orange). The hidden `Flower_Emission_Points` mesh is a duplicate source and is not imported again. The live markers are used because moving a marker does not update Blender's embedded JSON or source mesh.

Each emitter's `position` is already in Belfast scene coordinates and is retained in `Scene.flowers.emitters` for a future petal simulation. The export also preserves the Blender position, quality, ID, and reference splat indices. Blender indices refer to its filtered point cloud; `ply_splat_index` refers to the original PLY.

Blender's cloud has a baked translation as well as a different up axis. The verified mapping is `[x + 0.219901681, z - 0.998482904, -y - 1.806712568]`. This was checked against 1,311 corresponding splats from the PLY, including all 312 marker reference splats; the maximum error was below 0.000003 scene units. Calibration samples and the source PLY hash are stored in the export.

The inspection balls keep Blender's radius of 0.0045 scene units and its In Front behavior: markers remain visible through geometry. Use the checkbox at the top right to compare with the underlying scan. The positions are heuristic flower candidates, not verified flower centers. There is no petal simulation yet.

`pnpm --dir experiments/courtyard02 build` builds WASM and the site. Per the project owner's instruction, do not run tests for this experiment unless explicitly requested.
