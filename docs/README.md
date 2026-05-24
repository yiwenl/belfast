# Belfast documentation

Library API reference and guides for the WebGPU `belfast` package.

## Contents

| Doc                               | Description                                            |
| --------------------------------- | ------------------------------------------------------ |
| [Overview](overview.md)           | How the pieces fit together and a minimal render loop  |
| [API index](api/README.md)        | Public exports from `belfast`                          |
| [Feedback & Roadmap](feedback.md) | Architectural feedback and porting roadmap from Alfrid |

### API reference

| Module                          | Description                                |
| ------------------------------- | ------------------------------------------ |
| [Device](api/Device.md)         | Canvas, adapter, and `GPUDevice` setup     |
| [Draw](api/Draw.md)             | WGSL shader → render pipeline → draw call  |
| [RenderPass](api/RenderPass.md) | Begin a color render pass on the swapchain |
| [Utilities](api/utilities.md)   | WebGPU support checks and fallback UI      |

## Related

- [Triangle example](../examples/triangle/src/main.ts) — minimal usage
- [Restructure plan](../notes/webgpu-restructure-plan.md) — repo and tooling notes
- [alfrid reference](../packages/alfrid/src/) — legacy WebGL source (not part of `belfast` API)

## Contributing to docs

Add a new file under `docs/api/` when you export a new public symbol from [`packages/belfast/src/index.ts`](../packages/belfast/src/index.ts), then link it from [`api/README.md`](api/README.md).
