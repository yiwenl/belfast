# Blocks 1

Rust/WebGPU port of `Sketches/experiments/blocks1`, started from Belfast's
`experiments/template`. 65,536 instanced cubes follow a 128×128 GPU fluid field.
The original background and color-map JPGs are retained under `assets/` and
embedded into Wasm. No camera permission, hand tracker or pointer interaction.

## Run

```sh
pnpm install
pnpm --dir experiments/blocks1 dev
```

Open <http://localhost:5184>. Use a WebGPU-capable browser on localhost or HTTPS.
The template watcher rebuilds Rust/WGSL changes in this experiment and Belfast.

```sh
pnpm --dir experiments/blocks1 build     # release Wasm + TypeScript + Vite
pnpm --dir experiments/blocks1 preview   # http://localhost:4184
```

Tweakpane controls flow strength/radius, velocity and density retention, pressure
iterations, particle response, depth motion, block size, shadows, pause and reset.
Reset restarts the particles, fluid and emitter clock while retaining settings.
On small screens the pane starts collapsed. Pixel ratio is capped at 1.5.

`Fluid → Debug maps` toggles the bottom-left velocity and density previews (on by
default). Velocity hue shows direction and brightness shows speed using
`1 - exp(-speed)`; density is grayscale from 0 to 2. Both previews read the live
GPU field directly, with no CPU readback.

## Structure

- `src/scene.rs`: fixed 60 Hz simulation (at most four catch-up steps), autonomous
  emitters, parameters and camera. Edit `emitters` to change the flow pattern.
- `src/particles.rs`, `particle-update.wgsl`: GPU storage ping-pong, life/respawn,
  fluid sampling and depth noise. No per-frame particle transfers to JavaScript.
- `src/renderer.rs`, `blocks.wgsl`: flow-aligned cubes and 2048² PCF shadows.
- `src/browser.rs`: template canvas/surface lifecycle and the small Wasm API.
- `web/controls.ts`: Tweakpane. JS only forwards settings and frame timestamps.
- `../../crates/belfast/src/fluid/`: reusable compute fluid solver.

## Reuse the fluid solver

The shared module has no blocks-specific emitters, camera or rendering. Create it
once, bind `fluid.field()` as a read-only storage buffer in your particle shader,
and encode simulation before the consumer:

```rust
use belfast::fluid::{FluidSettings, FluidSimulation, FluidSplat};

let mut fluid = FluidSimulation::new(&device, 128, FluidSettings::default())?;
let dt = 1.0 / 60.0;
let impulses = [FluidSplat {
    position: [0.5, 0.5],
    velocity: [10.0 * dt, 0.0],
    radius: 0.1,
    density: 3.0 * dt,
}];
fluid.step(&device, &mut encoder, dt, &impulses)?;
// Encode your particle compute/render pass here, then submit the encoder.
```

`field()` is a stable row-major buffer of WGSL `array<vec4<f32>>` cells containing
`(velocity_u, velocity_v, 0, density)`. Coordinates are normalized UV with Y up;
velocity is UV/second. Sample bilinearly for particles (see `sample_fluid`).
Splats are impulses: multiply continuous emitter rates by dt. Up to 32 splats are
accepted per step, with dt in `0..=1/30`. The fixed grid size is selected at creation.

The solver performs semi-Lagrangian advection, injection, divergence, Jacobi
pressure and pressure projection entirely on the GPU. Walls block normal flow.
Retention is measured per 1/60 second. Pressure starts at zero each step; field
and pressure reset with `fluid.reset(&mut encoder)`. Several steps can share an
encoder: each step's parameter upload is ordered with its dispatches.

This is an artistic stable-fluid approximation: advection is numerically
dissipative, velocity is limited to 20 UV/second and density to 2 for stability.
Original WebGL noise, boundary behavior and frame-based integration are adapted,
so the motion is similar rather than numerically identical.

Per project direction this experiment is developed without further test runs;
verification is compilation and browser inspection.
