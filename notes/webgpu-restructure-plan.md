# Belfast WebGPU Restructure Plan

> Saved from planning session — May 2026

## Overview

pnpm monorepo with:

- **`packages/belfast`** — greenfield WebGPU library (TypeScript + Vite)
- **`packages/alfrid`** — WebGL reference source only (core classes, no examples or build tooling)
- **`examples/*`** — WebGPU examples (each is its own workspace package)

## Current scope (step 1)

- `Device` — canvas + WebGPU context
- `Draw` — render pipeline from WGSL
- `beginRenderPass` — simple color pass helper
- Triangle example with HMR via Vite

Not yet: camera, loaders, math utilities, additional examples beyond triangle.

## Dev commands

```bash
pnpm install
pnpm dev:all                      # default: library watch + triangle
pnpm dev:example                  # run default example (triangle)
pnpm dev:example textured-quad    # run a specific example
pnpm dev:all textured-quad        # library watch + specific example
pnpm examples                     # list available examples
```

Direct pnpm filters also work:

```bash
pnpm --filter @belfast/example-triangle dev
pnpm --parallel --filter belfast --filter @belfast/example-triangle dev
pnpm --filter "./examples/*" build
```

## Adding a new example

1. Create `examples/<name>/` with `package.json` named `@belfast/example-<name>`
2. Add `vite.config.ts` using [`examples/shared/vite.config.base.ts`](../examples/shared/vite.config.base.ts)
3. Run `pnpm dev:example <name>` — no root script changes needed

The workspace glob `examples/*` auto-discovers new examples. Root dev commands are parameterized via [`scripts/dev-example.mjs`](../scripts/dev-example.mjs) (default example: `triangle`).

## alfrid reference

Browse `packages/alfrid/src/` when porting features. No build step — source reference only.

Old WebGL examples removed to keep the repo focused on belfast step-by-step development.
