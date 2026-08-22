# Belfast examples

Rust + `wgpu` experiments that share [`common/`](common/mod.rs). Each file is a cargo example: `impl Example`, then `common::run`. Native window and browser wasm use the same source.

Run all commands from the repo root.

## Run native

```bash
cargo run -p belfast --example triangle
```

| Example             | What it shows                                      |
| ------------------- | -------------------------------------------------- |
| `triangle`          | One mesh, one draw                                 |
| `colored_triangle`  | Per-vertex colors                                  |
| `camera_uniform`    | Perspective camera + uniform bind group            |
| `texture`           | Sampled RGBA8 plane                                |
| `render_to_texture` | Offscreen pass, then present                       |
| `template`          | Orbit camera, axis helper — copy this to start new |
| `compute`           | Compute writes triangle vertices, then Draw        |
| `hdr_display`       | Linear luminance ramp on an HDR swapchain          |
| `particles`         | 10k instanced billboard discs, orbit camera        |

`hdr_display` asks for `rgba16float` plus an extended color space (`ExtendedSrgbLinear` on native, `ExtendedSrgb` on the web). It needs an HDR display (macOS: System Settings → Displays → High Dynamic Range). A red tick marks SDR white (`1.0`); the right side of the ramp is brighter-than-white when HDR presentation is active. On an SDR display the right side clips — that is the fallback.

## Run in the browser

Same Rust file, compiled to `wasm32-unknown-unknown`:

```bash
./scripts/wasm-example.sh triangle
```

Opens [http://127.0.0.1:8080](http://127.0.0.1:8080). Needs Chrome 113+, Edge 113+, or Safari 18+.

To upload a folder (HTML + JS + wasm already wired):

```bash
./scripts/build-example-wasm.sh triangle
```

Writes `dist/examples/triangle/` (`index.html`, `index.js`, `index_bg.wasm`). Copy that whole folder to the server. The server must serve `.wasm` as `application/wasm`.

Both scripts `--release` build: debug wasm is too slow for WebGPU.

If `wasm-bindgen` is not on `PATH`, the script uses the wasm-pack cache for the `Cargo.lock` version. Missing both: the script prints `cargo install wasm-bindgen-cli --version …`.

## Build only

```bash
# native → target/debug/examples/triangle
cargo build -p belfast --example triangle

# wasm binary only → target/wasm32-unknown-unknown/release/examples/triangle.wasm
# This is not a webpage. There is no index.js yet.
cargo build -p belfast --example triangle --target wasm32-unknown-unknown --release
cargo check -p belfast --examples --target wasm32-unknown-unknown
```

[`web/index.html`](web/index.html) is a static template. A deployable page is `./scripts/build-example-wasm.sh triangle` → `dist/examples/triangle/`.

## New experiment

```bash
cp crates/belfast/examples/template.rs crates/belfast/examples/my_experiment.rs
cargo run -p belfast --example my_experiment
./scripts/wasm-example.sh my_experiment
```

Keep using `common::run`. Do not add a new HTML page; [`web/index.html`](web/index.html) is shared.

The TypeScript gallery (`pnpm dev`, `@belfast/wasm`) is a different stack. These examples talk to the `belfast` crate directly.
