# Rust experiment template

A complete Rust-owned scene hosted by Vite + TypeScript. Starts with an axis helper and an orbital camera. No camera permission, Kinetic runtime, particles, or JS rendering engine is needed.

## Run

From the repository root, with Rust's `wasm32-unknown-unknown` target, `wasm-pack`, Node and pnpm installed:

```sh
pnpm install
pnpm --dir experiments/template dev
```

Open <http://localhost:5174>. WebGPU requires a supported browser on localhost or HTTPS.

- Left drag: orbit.
- Shift + left drag or middle drag: pan.
- Wheel / trackpad scroll: zoom.

`dev` builds release Wasm, starts Vite, and watches `.rs` / `.wgsl` under this experiment's `src/` and `crates/belfast/src`. A save rebuilds Wasm and reloads the page after a successful build. TypeScript and CSS keep Vite's hot reload. Cargo manifest changes are not watched; restart `dev` after changing dependencies.

```sh
pnpm --dir experiments/template watch:wasm  # rebuild Wasm only
pnpm --dir experiments/template build       # release Wasm + typecheck + static site
pnpm --dir experiments/template preview     # preview dist/
pnpm --dir experiments/template test
cargo test -p belfast-experiment-template
```

Deploy `dist/` as a static site. Wasm and its JS bindings are bundled into the build; there is no Rust server.

## Access from another computer on the same Wi-Fi

```sh
LAN_HOST=192.168.1.176 pnpm --dir experiments/template preview:lan
```

Replace `192.168.1.176` with the development computer's current Wi-Fi IP, then open `https://192.168.1.176:5175` on the other computer. This builds the site and serves only `dist/`, bound to the specified IP. With no `LAN_HOST`, preview stays on localhost. Vite's official basic SSL plugin generates a local development certificate cached under ignored `node_modules/`. The usual development server stays on localhost port 5174.

WebGPU requires a secure context, so a plain HTTP LAN address cannot run the renderer. The receiving browser will initially warn about the self-signed HTTPS certificate; handle that prompt yourself after checking the address is your development computer. If the browser still does not expose WebGPU, use a development certificate trusted on that computer. The receiving browser must also support WebGPU.

LAN preview has no hot reload. Rebuild with `pnpm --dir experiments/template build` after changes and refresh the other computer's browser.

If the connection times out, check that both devices are on the same network, the network allows device-to-device connections, and this computer's firewall permits the Node/Vite server. No router port forwarding is needed.

## Where to work

| File             | Responsibility                                                                               |
| ---------------- | -------------------------------------------------------------------------------------------- |
| `src/scene.rs`   | Rust experiment: camera, orbit, axes, update and render. Add simulation/GPU resources here.  |
| `src/browser.rs` | Canvas surface and exported Wasm API; resize and frame lifecycle.                            |
| `src/input.rs`   | Owned skeleton snapshot, validation and tracking timeout.                                    |
| `web/main.ts`    | Wasm startup, animation scheduling, errors and cleanup.                                      |
| `web/input.ts`   | DOM pointer/wheel events forwarded to Rust in CSS pixels.                                    |
| `web/kinetic.ts` | Optional Kinetic-compatible adapter; no runtime dependency.                                  |
| `scripts/`       | Watches `.rs` / `.wgsl` and rebuilds Wasm. `dev` also reloads Vite after a successful build. |
| `pkg/`           | Generated wasm-pack output. Do not edit or commit.                                           |

Rust owns every GPU resource, all camera calculations and all rendering. JavaScript calls one `frame()` per animation frame; it never fetches particle data or camera matrices.

## Public API

```ts
await init();
const app = await ExperimentApp.create(canvas);
app.resize(cssWidth, cssHeight, devicePixelRatio);
app.frame(performance.now());
app.pointerDown(x, y, button, shiftKey); // button 0 = primary, 1 = middle
app.pointerMove(x, y, cssWidth, cssHeight);
app.pointerUp(); // also use on cancel / blur
app.scroll(deltaInPixels);
app.setSkeleton(points, performance.now());
app.clearSkeleton();
app.free(); // stop RAF and input listeners first
```

`setSkeleton` copies a `Float32Array` with exactly 132 values: 33 MediaPipe landmarks in Kinetic's fixed order, each `[x, y, z, confidence]`. Coordinates must be finite; confidence must be in `[0, 1]`. Timestamps must be finite, non-negative, non-decreasing milliseconds from the same `performance.now()` clock as `frame()`. Malformed input throws without replacing the previous accepted frame. `clearSkeleton()` removes the pose immediately; `SkeletonInput.latest(now_ms)` returns no pose after 250 ms without an update.

The template only stores the pose. Read `self.skeleton.latest(now_ms)` in `Scene::update` when adding forces, emitters, or a GPU skeleton buffer.

## Connect Kinetic later

Add your local Kinetic package when needed, for example from this folder:

```sh
pnpm add kinetic@file:../../../kinetic
```

Inside `start()`, after `ExperimentApp.create(canvas)` and the existing `createSkeletonAdapter(created)` call, the wiring is:

```ts
// Add at the top of web/main.ts after installing the dependency:
import { SkeletonManager } from "kinetic";

// In start():
const tracker = new SkeletonManager({ mirror: true });
tracker.addEventListener("skeleton-detected", ({ detail }) => {
  skeleton.update(detail.poses, performance.now());
});
tracker.addEventListener("error", () => skeleton.clear());
await tracker.init();
```

Keep the tracker in the host's lifecycle state, and call `tracker.dispose()` before `app.free()` in `dispose()`. Also dispose the tracker if initialization finishes after the host has been disposed, matching the existing Wasm initialization guard. Start camera initialization from the user interaction appropriate to your experiment.

The adapter takes the first pose's `keypoints3D`, preserves Kinetic's already-mirrored X, and maps `[x, y, z]` to `[x, -y, -z]`: Y up and positive Z toward the initial viewer. One metre becomes one scene unit. Missing confidence becomes zero; empty/malformed poses clear tracking. Rust owns a copy, so the adapter reuses its packing buffer.

These are estimated hip-relative coordinates, not a calibrated position in the room. If the whole body should move across the scene, combine image-space hip position with the local 3D skeleton in this adapter. Kinetic currently performs synchronous inference on the main thread; moving it into a Worker is a separate optimization if profiling shows dropped frames.

## Start another experiment

Copy this directory without `pkg/`, `dist/`, or `node_modules/`, rename the Cargo package and npm package, and add the copied path to the root Cargo/pnpm workspace lists. Give its Vite server another port if running experiments together. The wasm output name can stay `experiment`; it is local to each directory.

For a standalone repository, replace workspace-inherited Cargo dependencies with explicit versions and a path or Git dependency on Belfast. This template currently uses the parent workspace's dependency versions.
