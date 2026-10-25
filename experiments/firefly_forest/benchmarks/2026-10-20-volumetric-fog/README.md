# Animated volumetric fog — 2026-10-20

First implementation on top of `37a96fa`, including the pending near-camera
splat fade (hidden below 0.5 forward units, fully visible at 1.0). All runs
use that same fade. No commit or optimization acceptance comparison against
the pre-fade renderer is implied.

## Implementation

- 480 × 270 ray march for a 1920 × 1080 canvas, 32 steps, maximum distance 32.
- Seeded 32³ R8 noise with two animated sampling scales and height falloff.
- Blue light at (0, 2.2, -10), facing the foreground; single scattering and
  transmittance. No light-space shadow map or firefly light-map interaction.
- Existing forest depth bounds the rays; four-tap depth-aware upsampling
  composites in the existing grading pass. Forest color/depth remain cached
  with a static camera; fog runs every frame.
- Local firefly lighting defaults off; original firefly glow remains.
- Fog settings are included in benchmark schema v6. Its clock resets and uses
  the fixed simulation step. Forest GPU span includes fog and composition.

## Measurement

Browser: local Codex in-app browser, Chromium 155 on macOS, HDR
Rgba16Float / ExtendedSrgb, merged occlusion enabled. The browser did not expose
an adapter name in the exported environment string.

Each run: 120 warmup frames + 1,200 measured frames, seed 1831565813,
dt 1/60, `forest-loop-v1`, 10,000 fireflies, movement 0.27, frequency 0.28,
flash duration 1.5 seconds, FOV 40°, offscreen simplification on, local lights off.
CSS size 833 × 469, DPR 2; physical buffer remains 1920 × 1080.

| Run (chronological)                       | Fog density / speed / light |     FPS | Frame mean ms | GPU total mean ms | GPU total P95 ms |
| ----------------------------------------- | --------------------------- | ------: | ------------: | ----------------: | ---------------: |
| [Initial off reference](off-initial.json) | off; 0.06 / 1 / 0.7         | 71.1069 |       14.0633 |           12.3266 |          14.3524 |
| [Fog on](on.json)                         | on; 0.14 / 1.5 / 0.95       | 65.9192 |       15.1701 |           13.2936 |          16.3185 |
| [Matched fog off](off.json)               | off; 0.14 / 1.5 / 0.95      | 68.7344 |       14.5488 |           12.7989 |          15.3354 |

The user adjusted the live fog controls between the initial reference and the
paired runs. The on/off pair uses those live values, not the source defaults
(0.06 / 1 / 0.7). Only `config.fog.enabled` differs within that pair; exported
camera matrices match at every sampled frame. All runs contain 1,200 RAF
intervals, 1,200 sets of GPU timings and camera samples along the route.

Paired cost: **2.8152 FPS**, **0.6213 ms mean frame time**, and **0.4947 ms
whole-frame GPU time**. This meets the approximate 10 FPS budget and exceeds
60 FPS on this route/configuration. It is one matched pair, not a general
guarantee. The initial off run differs by 2.37 FPS from the later off run,
showing normal run-to-run variation. Against that earlier reference, fog costs
5.19 FPS. Different views, larger particle populations, other GPUs and enabling
local lighting can change the result. Individual GPU pass spans overlap and
must not be summed to infer whole-frame duration.

## Verification and remaining limits

Release Wasm, TypeScript and Vite build passed. Final default-intensity change
also passed the watcher release build. Browser rendered both fog modes and
completed GPU readback; no test suite was run. The local browser used the
merged-depth path; the separate-depth fallback was not runtime-checked here.

The front opening illuminates animated mist while nearby tree silhouettes
remain dark. Initial light intensity 2 was visually too bright and was reduced
to 0.7 for the default. Fog is an artistic display-RGB approximation without
light-space shadows, multiple scattering or fog extinction on firefly sprites.
Thin geometry can expose depth-upsample artifacts. Existing saved baselines
were not overwritten. The live fog values and camera were restored after
measurement.
