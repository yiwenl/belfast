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
docs/          Library API reference and guides
notes/         Planning docs
```

## Commands

```bash
pnpm build                        # build belfast library
pnpm dev                          # watch-build belfast
pnpm dev:example                  # run default example (triangle)
pnpm dev:example textured-quad    # run a specific example
pnpm dev:all                      # library watch + default example
pnpm dev:all textured-quad        # library watch + specific example
pnpm examples                     # list available examples
pnpm typecheck
pnpm format                       # format all files
pnpm format:check                 # check formatting (CI)
```

## WebGPU browser support

Requires Chrome 113+, Edge 113+, or Safari 18+.

## Next steps

Build incrementally from the triangle example: textured quad, buffers, then port ideas from `packages/alfrid/src/` as needed.

- [Library docs](docs/README.md) — API reference and overview
- [Restructure plan](notes/webgpu-restructure-plan.md) — repo and tooling notes
