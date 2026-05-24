# Belfast documentation

Library API reference and guides for the WebGPU `belfast` package.

## Contents

| Doc                                                                     | Description                                                      |
| ----------------------------------------------------------------------- | ---------------------------------------------------------------- |
| [Overview](overview.md)                                                 | How the pieces fit together and a minimal render loop            |
| [API index](api/README.md)                                              | Public exports from `belfast`                                    |
| [Feedback & Roadmap](feedback.md)                                       | Architectural feedback and porting roadmap from Alfrid           |
| [Cursor ↔ Antigravity workflow](workflows/cursor-antigravity-review.md) | Build in Cursor, review in Antigravity, refine in Cursor         |
| [MCP review bridge v2 plan](workflows/mcp-review-bridge-v2-plan.md)     | MCP server design and setup (implemented in `mcp/`)              |
| [Feature reviews](reviews/)                                             | Per-feature Antigravity feedback (`<slug>-antigravity.md`)       |
| [Feature specs](features/)                                              | Per-feature design and focused feedback (`vertex-buffers.md`, …) |

### API reference

| Module                          | Description                                |
| ------------------------------- | ------------------------------------------ |
| [Device](api/Device.md)         | Canvas, adapter, and `GPUDevice` setup     |
| [Buffer](api/Buffer.md)         | Shareable GPU buffer                       |
| [Mesh](api/Mesh.md)             | Vertex buffer bindings + layouts           |
| [Draw](api/Draw.md)             | WGSL shader → render pipeline → draw call  |
| [RenderPass](api/RenderPass.md) | Begin a color render pass on the swapchain |
| [Utilities](api/utilities.md)   | WebGPU support checks and fallback UI      |

## Related

- [Triangle example](../examples/triangle/src/main.ts) — minimal usage
- [Restructure plan](../notes/webgpu-restructure-plan.md) — repo and tooling notes
- [alfrid reference](../packages/alfrid/src/) — legacy WebGL source (not part of `belfast` API)

## Contributing to docs

When adding a library feature:

1. Add `docs/features/<slug>.md` (design, usage, feature-local feedback section)
2. Add or update `docs/api/<Symbol>.md` and [`api/README.md`](api/README.md)
3. Submit review handoff: `docs/reviews/queue/<slug>.json` + `docs/reviews/queue/<slug>-implementation.md`
