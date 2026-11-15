# Mountain Slice

A copy of Mountain Fold with terrain-following diagonal slices. The seeded mountain generation, hydraulic erosion and shaded mesh are retained. The default line mode keeps a uniform gap across the terrain and removes segments in cast shadows or on backlit slopes. Closed cross-sections can also be previewed in 3D and exported as physical cutting templates.

The default footprint is 12 × 6 world units: X remains −6…6 and Z is cropped to −3…3. Height generation and erosion still run on the original 12 × 12 field, so the retained mountain is not compressed or resampled. The mesh, base walls, lines, shadows and cutting templates all use the cropped boundary; the camera framing and line spacing stay unchanged.

## Run

```sh
pnpm --dir experiments/mountain_slice dev
```

Open <http://localhost:5186>. Rust/WGSL edits rebuild Wasm and reload the page; TypeScript/CSS use Vite hot reload. The experiment has its own package names and ports so Mountain Fold can run alongside it. WebGPU requires a supported browser on localhost or HTTPS.

## Controls

- **Light & lines → Slice angle (°):** rotates the slices in the terrain’s XZ plane, from −90° to 90° (default −55°). At 0°/90° they follow the base edges; other angles cut diagonally. Changing the angle does not reset erosion.
- **Line density:** ranges from 10 to 320 in steps of 1 and sets the gap to 12 world units divided by the density (default 128, giving 0.09375 world units). The gap stays constant as the angle, light or camera changes; projection and terrain slope can change its apparent screen spacing.
- **Light azimuth / elevation:** move the light and the gaps cut by its shadows (defaults −39° / 26°). Lit areas keep every slice; HDR brightness, material color and exposure do not thin out the slices. Line exposure, Ink on light and wobble controls are hidden in this mode.
- **Line render:** defaults to **Diagonal slices**. The copied Terrain curves, Straight hatching and Voronoi hatching modes remain available for comparison.
- **Scene → SVG preview:** on by default, showing just the lines on light paper. **Dark paper** swaps to light lines on dark paper. Orbit, pan and zoom remain live. The captured startup camera looks toward the origin at about 52.298° orbit / 23.981° elevation, with orthographic bounds ±7.0481. **Reset camera** restores this view.
- **Scene → Section preview:** shows every diagonal slice as a closed 3D outline: full terrain profile, both sides down to the base bottom, and a flat bottom joining them. Shadows, facing and hidden-surface removal do not cut these paths. Switching preserves the current camera pose, zoom and paper framing. Angle, density, erosion and base depth update live. Turn it off to return to the shaded slices.
- **Scene → Z range:** retains the centered fraction of the original Z extent, from 0.5 to 1 in steps of 0.01 (default 0.5). At 0.5 the footprint is 12 × 6; at 1 it is the full 12 × 12. Updates live without restarting erosion, moving the camera or squeezing the mountain.
- **Scene → Layer spacing:** separates the lines from the shaded mesh for comparison outside SVG preview.
- **Cut sections → Material (mm):** read-only material thickness needed to reproduce the 3D mountain at the print scale. Every slice is retained. The widest slice is scaled to 80% of A4's 210 mm width (168 mm), and all pieces use that same scale. The required thickness is the original slice spacing at this scale; changing density changes this value. **Model X × Z (mm)** shows the resulting model footprint. Material thickness no longer sets the output size.
- **Cut sections → Base depth (world):** depth below terrain zero, default 0.6 world units. It controls the closed profiles' bottom and the solid mesh base.
- **Cut sections → Print A4 pages:** opens the browser's print dialog with numbered, portrait A4 pages (210 × 297 mm). Choose a printer or Save as PDF. Closed profiles are packed in slice order, left to right and then top to bottom, with 21 mm page margins and 4 mm gaps between pieces. The 168 mm widest-slice target and uniform scale apply across every page. Each piece stays on one page; if a profile is too tall, the output asks for a lower Base depth or Mountain height instead of clipping or distorting it. Print at 100% / actual size on A4 paper. The Export field shows the slice/page count or an error.
- **Cut labels:** each piece has its slice number and `x=` horizontal start coordinate in millimetres relative to the shared model origin, so packing does not lose assembly alignment. Every page lists the required material thickness and page number. Printing uses a snapshot of the current terrain (or initial terrain when Before erosion is enabled), independent of camera, shadows and preview mode.
- **Export line SVG:** uses the same clipped segments as the preview. The angle and density are included in its metadata. Ctrl/Cmd+S saves a PNG.
- Press `/` or `\` to switch between the compact and full controls. Mountain generation, erosion and camera controls work as in [Mountain Fold](../mountain_fold/README.md).

## Slicing

Parallel lines cover the original terrain square’s full diagonal at fixed perpendicular spacing. Each is split into 1,024 short segments, lifted onto the current height field and clipped against the cropped terrain boundary, camera depth, back-facing slopes and sunlight visibility. A shadow removes ink entirely instead of changing line density. Visibility transitions are refined within each segment, and SVG paths join only consecutive visible segments from the same slice.

The existing finite-step terrain shadow ray and finite segment sampling are retained; extremely small shadow details can fall between samples. Visual verification is manual.

Closed-section preview and printing share the same rectangle-clipped slice spans. Each complete top edge has 1,024 segments sampled with the terrain mesh's triangle interpolation, followed by the two side edges and bottom. Print pages use vector SVG paths ending in `Z`, so outlines remain closed and sharp on paper or in a PDF. The 3D preview and controls are excluded from printing.
