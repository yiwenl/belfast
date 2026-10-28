# Blocks1 Rust/WebGPU port

Port Sketches/experiments/blocks1 into a copy of experiments/template at
experiments/blocks1. Keep the original 256² instanced cubes, color map,
background, life/respawn behavior, flow alignment and self shadows. Rust owns
simulation and rendering; TypeScript owns startup, FPS and Tweakpane.

The requested first phase has autonomous random flow, no hand detection and no
pointer forces. The camera starts front-on as in the source. Tweakpane controls
flow strength/radius, dissipation, pressure iterations, particle response/size,
depth motion, pause and reset. Defaults should resemble the source.

## Approaches

- Keep fluid local: less public API, but other experiments must copy it.
- Put a small `belfast::fluid` module in the shared crate (chosen): reuse the
  existing Compute/Buffer helpers; expose settings, splats, step and a stable GPU
  field buffer. The caller supplies forces, so no blocks-specific behavior leaks.
- A separate simulation crate adds packaging overhead without another consumer.

Implementation choice: a 128² compute-based stable-fluid grid with bilinear
advection, splats, divergence, Jacobi pressure and projection. Packed field cells
are vec4<f32>(velocity in UV/second, zero, density). Pressure has separate buffers.
Particles read the field on the GPU and ping-pong their own storage buffers.
Use bounded fixed 1/60-second steps for repeatable motion across refresh rates.
Only settings and eight emitters cross the CPU/GPU boundary per step.

## Verification

Check numerical GPU readback (finite values, injected flow, pressure reducing
divergence, reset), shader validation, particle respawn and rendering, Wasm and
Vite production builds, and an actual browser run including GUI changes/resize.
Native GPU checks explicitly require an adapter; ordinary validation tests do not.
