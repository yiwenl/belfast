# @belfast/wasm

WebAssembly bindings for the Belfast wgpu renderer.

```bash
npm install @belfast/wasm
```

```ts
import init, { Device, Draw } from "@belfast/wasm";

await init();
const device = await Device.create(canvas);
```

Rebuild the generated package from the repository root with `pnpm build:wasm`.
