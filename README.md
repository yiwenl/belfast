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

pnpm build:wasm    # wasm-pack → packages/belfast-wasm/dist
pnpm pack:check    # npm pack --dry-run for @belfast/wasm
pnpm format
pnpm format:check
```

Rust examples: [`crates/belfast/examples/README.md`](crates/belfast/examples/README.md).

The TypeScript gallery (`pnpm dev`) is at `/`. Basic examples use `/basic/?example=colored-triangle`; standalone pages live at `/template/` and `/instancing/`.

## WebGPU browser support

Requires Chrome 113+, Edge 113+, or Safari 18+.

## Docs

- [Rust engine docs](docs/README.md)
- [Archived TypeScript API](archive/docs/README.md)
