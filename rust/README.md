# Belfast Rust

Rust implementation of Belfast built on `wgpu`. This workspace is independent from the existing TypeScript package at `packages/belfast`.

## Workspace

- `crates/belfast`: reusable native and WebAssembly-compatible rendering API.
- `crates/belfast-wasm`: JavaScript-facing `wasm-bindgen` facade.
- `docs`: Rust architecture and API parity notes.

## Commands

Run these commands from this directory:

```bash
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p belfast-wasm --target wasm32-unknown-unknown
wasm-pack build crates/belfast-wasm --target web --out-dir ../../pkg/belfast-wasm
```

Native examples use Cargo's standard example targets:

```bash
cargo run -p belfast --example triangle
cargo run -p belfast --example colored_triangle
cargo run -p belfast --example camera_uniform
cargo run -p belfast --example texture
cargo run -p belfast --example render_to_texture
cargo run -p belfast --example template
```

Start a small experiment by copying the template from the `rust` directory:

```bash
cp crates/belfast/examples/template.rs crates/belfast/examples/my_experiment.rs
cargo run -p belfast --example my_experiment
```

The `wasm-pack` command writes a generated npm package to `rust/pkg/belfast-wasm`. That directory is ignored by Git and contains no handwritten Belfast source.

## Web examples

Build the generated WebAssembly package before installing or building the browser gallery:

```bash
cd rust
wasm-pack build crates/belfast-wasm --target web --out-dir ../../pkg/belfast-wasm
cd ..
pnpm install
pnpm --filter @belfast/rust-wasm-examples dev
```

The landing page lists examples at `/`. The basic gallery's colored triangle is available at `/basic/?example=colored-triangle` (for example, `http://127.0.0.1:5173/basic/?example=colored-triangle` when Vite uses its default port). The compute triangle is available at `/basic/?example=compute-triangle`. The orbit template is available at `/template/`.

Add basic gallery modules in `rust/web/examples/basic/src/examples` and their WGSL shaders in `rust/web/examples/basic/src/shaders`. Standalone experiments live in sibling folders such as `rust/web/examples/template`.
