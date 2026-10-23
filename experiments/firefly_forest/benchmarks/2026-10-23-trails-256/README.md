# 256-frame trail performance

Measured on 2026-10-23 in the local in-app browser at 1920 × 1080, HDR
`Rgba16Float / ExtendedSrgb`. The browser did not expose an adapter name.
Each case is one 1,200-frame run with seed 1831565813, 120 warmup frames,
fixed simulation steps of 1/60 second, and offscreen simplification enabled.
Movement is 0.27×, thickness 0.75×; all other controls retain their defaults.

| Particles | Camera | Average FPS | Frame P95 ms | GPU total mean ms | Trail draw mean ms | Raw result                 |
| --------- | ------ | ----------: | -----------: | ----------------: | -----------------: | -------------------------- |
| 20,000    | Static |       94.80 |        16.80 |              7.72 |               6.78 | [JSON](20000-static.json)  |
| 20,000    | Moving |       65.29 |        20.10 |             13.23 |               8.54 | [JSON](20000-moving.json)  |
| 100,000   | Static |       21.05 |        53.90 |             45.14 |              33.80 | [JSON](100000-static.json) |
| 100,000   | Moving |       19.91 |        56.40 |             47.90 |              35.94 | [JSON](100000-moving.json) |

Moving runs use the complete `forest-loop-v1` route; static runs use the initial
interactive camera. Different camera paths also see different scene contents.
The reported GPU component timings do not sum to the total span; they are
preserved as measured, without treating them as an additive breakdown. FPS also
includes presentation and browser scheduling.

The benchmark resets history when measurement starts. Excluding the first 256
frames while it fills gives 94.16 / 65.56 FPS at 20,000 particles and
20.95 / 19.86 FPS at 100,000 particles (static / moving), so the conclusion is
unchanged: 20,000 remains usable, while 100,000 cannot sustain 60 FPS.
These are single local runs, not a cross-device comparison or a measured
speed ratio against the earlier 64-frame version.

History reserves 409.6 MB for 100,000 particles even when fewer are active.
Four 102.4 MB buffers avoid the default per-buffer/binding limits. Ordinary
frames write only one slot in one chunk; initialization/count growth resets all
four chunks. The mesh has 256 triangular rings and 1,530 triangles per particle.
History still records once per render, so its duration changes with interactive
FPS. Fixed-time sampling was not part of this experiment.

Validation: 14 standard Rust tests, 3 focused GPU tests (including maximum
capacity, full-ring wrap/reset and solid depth-occluded rendering), 11 frontend
tests and the release WASM/TypeScript/Vite build passed. The browser reported
no console warnings or errors during these runs. Preview restored to 20,000
particles with 256-frame trails enabled.
