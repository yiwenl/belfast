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
