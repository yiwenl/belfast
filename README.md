# Belfast

Rust/`wgpu` rendering library with a WebAssembly package for the browser.

The TypeScript WebGPU library is frozen under [`archive/`](archive/).

## Quick start

Native:

```bash
cargo run -p belfast --example triangle
```

Same Rust example in the browser:

```bash
./scripts/wasm-example.sh triangle
```

TypeScript gallery (`@belfast/wasm`):

```bash
pnpm install
pnpm dev
```

Install the published WebAssembly package in another project with:

```bash
npm install @belfast/wasm
```

```ts
import init, { Device, Draw } from "@belfast/wasm";

await init();
const device = await Device.create(canvas);
```

## Repo layout

```
crates/belfast          Native wgpu engine
crates/belfast-wasm     wasm-bindgen facade
packages/belfast-wasm   npm package (@belfast/wasm)
web/                    Vite gallery
docs/                   Rust PRD and API parity
archive/                Frozen TypeScript WebGPU library
```

## Commands

```bash
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p belfast-wasm --target wasm32-unknown-unknown
cargo check -p belfast --examples --target wasm32-unknown-unknown

cargo run -p belfast --example triangle
cargo run -p belfast --example colored_triangle
cargo run -p belfast --example camera_uniform
cargo run -p belfast --example texture
cargo run -p belfast --example render_to_texture
cargo run -p belfast --example template
./scripts/wasm-example.sh triangle
./scripts/wasm-example.sh template
./scripts/build-example-wasm.sh triangle   # dist/examples/triangle/

pnpm build:wasm                         # wasm-pack → packages/belfast-wasm/dist
pnpm --filter @belfast/web-examples dev
pnpm pack:check                         # npm pack --dry-run for @belfast/wasm
pnpm format
pnpm format:check
```

Example run/build notes: [`crates/belfast/examples/README.md`](crates/belfast/examples/README.md).

Start an experiment from the template:

```bash
cp crates/belfast/examples/template.rs crates/belfast/examples/my_experiment.rs
cargo run -p belfast --example my_experiment
./scripts/wasm-example.sh my_experiment
```

The web gallery is at `/`. Basic examples use query strings such as `/basic/?example=colored-triangle`. Standalone pages live at `/template/` and `/instancing/`.

Add basic gallery modules in `web/basic/src/examples` and their WGSL shaders in `web/basic/src/shaders`.

## WebGPU browser support

Requires Chrome 113+, Edge 113+, or Safari 18+.

## Docs

- [Rust engine docs](docs/README.md)
- [Archived TypeScript API](archive/docs/README.md)
