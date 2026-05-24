# Belfast WebGPU Restructure Plan

> Saved from planning session — May 2026

## Overview

pnpm monorepo with:

- **`packages/belfast`** — greenfield WebGPU library (TypeScript + Vite)
- **`packages/alfrid`** — WebGL reference source only (core classes, no examples or build tooling)
- **`examples/triangle`** — first example: draw a single triangle

## Current scope (step 1)

- `Device` — canvas + WebGPU context
- `Draw` — render pipeline from WGSL
- `beginRenderPass` — simple color pass helper
- Triangle example with HMR via Vite

Not yet: camera, loaders, math utilities, additional examples.

## Dev commands

```bash
pnpm install
pnpm dev:all
```

## alfrid reference

Browse `packages/alfrid/src/` when porting features. No build step — source reference only.

Old WebGL examples removed to keep the repo focused on belfast step-by-step development.
