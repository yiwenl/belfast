# Blocks 1

Rust/WebGPU port of `Sketches/experiments/blocks1`, started from Belfast's
`experiments/template`. 65,536 instanced cubes follow a 128×128 GPU fluid field.
The original background and color-map JPGs are retained under `assets/` and
embedded into Wasm. No camera permission or hand tracker.

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

### Same-Wi-Fi preview

```sh
pnpm --dir experiments/blocks1 preview:lan  # build once + HTTPS preview
pnpm --dir experiments/blocks1 watch:lan   # build + HTTPS preview + Rust/WGSL watcher
```

Run one of these commands, then open the printed Network URL on another computer
on the same Wi-Fi, for example `https://192.168.1.102:5185`. LAN preview binds to
all IPv4 interfaces by default and serves the compiled `dist/` directory. To bind
only the Wi-Fi interface, set its current IP explicitly:

```sh
LAN_HOST=192.168.1.102 pnpm --dir experiments/blocks1 watch:lan
```

`watch:lan` rebuilds release Wasm, checks TypeScript and bundles the site whenever
`.rs` or `.wgsl` changes under this experiment's `src/` or `crates/belfast/src/`.
Rapid saves are combined and builds run one at a time. After `Build #N complete`,
refresh the remote browser to load the new version; this static preview has no
hot reload. Build errors are printed and the next source save retries. Restart
the command after changing Cargo manifests, web files, assets or configuration.
Stop `dev` / `watch:wasm` before using `watch:lan` so two watchers do not write to
the same generated `pkg/` at once. Ctrl+C stops the watcher and preview server.

WebGPU needs a secure context, so use the **HTTPS** URL. The existing Vite basic
SSL plugin generates a self-signed development certificate in ignored
`node_modules/`. The receiving browser may show a certificate warning; review it
yourself after checking the address is your development computer. If WebGPU
remains unavailable, use a certificate trusted by that computer and a browser
that supports WebGPU. Both computers must be on a network that allows local
device connections, and the host firewall must allow Node on port 5185.

### Controls

Tweakpane controls flow strength/radius, velocity/density/pressure retention, pressure
iterations, particle response, depth motion, block size, shadows, pause and reset.
Reset restarts the particles, fluid and emitter clock while retaining settings.
On small screens the pane starts collapsed. Pixel ratio is capped at 1.5.

Press `F` to toggle fullscreen, or `R` to generate a random preset without opening
the dialog. Generation errors open the dialog so you can retry. Shortcuts ignore
text fields, editable content, held keys, IME composition and Ctrl/Cmd/Alt combinations.
`R` is ignored while an image is generating or colors are still blending.

`Generate colors…` opens a text dialog. Enter an English scene description and
submit it to the SD-Turbo service at `http://192.168.1.201:8189`. `Random` fills in
one of 10 preset scene prompts and immediately generates it. You can edit the
selected prompt before generating another image. Particles keep
moving while it generates a 512×512 PNG. Once the request completes, the image is
decoded as an ImageBitmap and uploaded to WebGPU as the next particle palette,
using fixed random UVs hashed from each particle's ID, spread over the full image.
The UVs are independent of particle position and persist through movement and
respawns, so colors do not reconstruct the image or flicker. The old and new
palettes use the same UVs and blend during a two-second transition. A shared uniform offset advances from
0 to 1; each particle uses `t = clamp(offset * 2 - rnd, 0, 1)` to stagger its blend.
The fixed `rnd` lives in the unused `velocity.w` slot and survives compute updates
and respawns, so the particle buffer stays 64 bytes per particle. Color blending
continues even when particle motion is paused.

Each generated image also becomes the background: a one-time GPU Gaussian blur
creates a 64×64 texture, with a blur sigma of 1/8 of the source width (64 pixels
for a 512×512 image). Clamp-to-edge sampling avoids dark borders. The blur runs
in linear color and lowers brightness by two stops to keep particles visible.
The background fades with the same two-second transition; rendering each frame
only samples the cached blurred textures. The original background remains until
the first image arrives, and reset keeps the current background.

The dialog closes automatically once the generated image is loaded and the blend
begins. Reopen it to view the image preview. Submission stays locked until the
blend finishes so neither active palette is replaced midway through a transition.
There is one request at a time, with up to two minutes for initial model loading;
errors keep the previous palette and allow a manual retry. Closing the dialog
leaves an active request running; unloading the app cancels that client's request.
Reset simulation keeps the current palette; reloading restores `colorMap.jpg`.

Vite dev and preview proxy `/image-api/` HTTP and WebSocket requests to the image
service so HTTPS LAN clients avoid mixed-content and CORS restrictions. Only
the workflow, queue, prompt, interrupt and WebSocket routes are forwarded. Use
`dev`, `preview`, `preview:lan` or `watch:lan`; a separately hosted static `dist/`
needs the same proxy routes. The host running Vite must be able to reach port 8189. Change the target in `vite.config.ts` and restart if the service IP changes.

Move the mouse over the canvas to push the fluid; no button press is needed.
`Mouse → Strength` controls the impulse (zero disables it), and `Mouse → Radius`
sets the radius in fluid UV units. Movement is projected onto the z=0 particle
plane and spread along each stroke. The fluid moves the particles through their
existing compute shader. Hovering over the GUI does not push the fluid. Leaving
the canvas, pausing, resizing, resetting or switching away clears pointer state.

`Fluid → Debug maps` toggles the bottom-left velocity and density previews (off by
default). Velocity displays X in red, Y in green and zero blue using
`RG = clamp(0.5 + velocity.xy * 0.5, 0, 1)`: each channel maps -1..1 UV/second
to 0..1, with zero velocity at 0.5. Density is grayscale from 0 to 2. Both previews read the live
GPU field directly, with no CPU readback.

## Structure

- `src/scene.rs`: fixed 60 Hz simulation (at most four catch-up steps), autonomous
  emitters, parameters and camera. Edit `emitters` to change the flow pattern.
- `src/particles.rs`, `particle-update.wgsl`: GPU storage ping-pong, life/respawn,
  fluid sampling and depth noise. No per-frame particle transfers to JavaScript.
- `src/renderer.rs`, `blocks.wgsl`: flow-aligned cubes and 2048² PCF shadows.
- `src/browser.rs`: template canvas/surface lifecycle and the small Wasm API.
- `web/controls.ts`: Tweakpane. JS forwards settings, pointer events and frame timestamps.
- `web/input.ts`, `src/mouse.rs`: canvas pointer events, camera-plane projection
  and movement impulses consumed once per simulation step.
- `web/color-dialog.ts`: prompt UI, PNG decoding and palette updates.
- `web/particle-client.mjs`: client adapted from the image service's
  `/client.mjs`, retaining its prompt-scoped WebSocket and cancellation protocol
  with an added AbortSignal for page disposal. The workflow is fetched from the
  service on each submission. Generation starts only when the user submits.
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
velocity is UV/second. Sample bilinearly with mirrored-repeat addressing for
particles (see `sample_fluid`), including outside the 0..1 UV range.
Splats are impulses: multiply continuous emitter rates by dt. Up to 32 splats are
accepted per step, with dt in `0..=1/30`. The fixed grid size is selected at creation.

The solver performs semi-Lagrangian advection, injection, divergence, Jacobi
pressure and pressure projection entirely on the GPU. Field and pressure sampling
use mirrored-repeat addressing, matching the original WebGL textures. Mirroring
changes sample coordinates without reversing velocity components; edges do not
block normal flow, and particles can continue past the simulation domain.
Retention is measured per 1/60 second. Each pressure solve starts from the previous
solution multiplied by `pressure_dissipation`, matching the original WebGL solver;
the pressure-gradient scale also follows that source. This preserves coherent
vortices with the original 20 pressure iterations. Field and pressure are cleared
only by `fluid.reset(&mut encoder)`. Several steps can share an
encoder: each step's parameter upload is ordered with its dispatches.

This is an artistic stable-fluid approximation: advection is numerically
dissipative, velocity is limited to 20 UV/second and density to 2 for stability.
Original WebGL noise and frame-based integration are adapted,
so the motion is similar rather than numerically identical.

Per project direction this experiment is developed without further test runs;
verification is compilation and browser inspection.
