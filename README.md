# Belfast

WebGPU utility library with the legacy WebGL library (`alfrid`) kept in-repo as reference source.

## Quick start

```bash
pnpm install
pnpm dev:all
```

Opens the triangle example with the library in watch mode.

## Repo layout

```
packages/
  belfast/     WebGPU library (TypeScript + Vite)
  alfrid/      WebGL reference source (not built or published)
examples/
  triangle/    Hello triangle — first WebGPU example
notes/         Planning docs
```

## Commands

```bash
pnpm build          # build belfast library
pnpm dev            # watch-build belfast
pnpm dev:example    # run triangle example
pnpm dev:all        # library watch + triangle example
pnpm typecheck
```

## WebGPU browser support

Requires Chrome 113+, Edge 113+, or Safari 18+.

## Next steps

Build incrementally from the triangle example: textured quad, buffers, then port ideas from `packages/alfrid/src/` as needed.

See [`notes/webgpu-restructure-plan.md`](notes/webgpu-restructure-plan.md) for the full plan.
