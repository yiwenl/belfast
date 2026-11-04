# Vortex filaments

`belfast::filament` provides a reusable GPU vortex-ring simulation, following the `belfast::fluid` pattern. It owns the rings and their velocity sampler; particle state, rendering, cameras, emitters and UI belong to the caller. The courtyard experiment is one consumer.

```rust
use belfast::filament::{FilamentSettings, FilamentSimulation};

let settings = FilamentSettings {
    origin: [0.0, -0.2, 0.0],
    ring_radius: 0.21,
    core_radius: 0.036,
    ..Default::default()
};
let mut filaments = FilamentSimulation::new(&device, 6, 12, settings)?;

// Create consumer bindings once, using filaments.field() and filaments.params().
let mut encoder = device.gpu().create_command_encoder(&Default::default());
filaments.step(&device, &mut encoder, 1.0 / 60.0)?;
// Encode your particle update or debug drawing here, reading the updated field.
device.queue().submit([encoder.finish()]);
```

## API and ordering

- `new(device, rings, vertices_per_ring, settings)` seeds closed XZ rings in Y-up world coordinates. Ring count must be positive, each ring needs at least three vertices, and buffer sizes/dispatch counts must fit the device. The initial ring count sets the capacity.
- `set_ring_count(rings)` selects 1 through the original ring count. Changing the count reseeds the active rings on the next step and preserves the drift clock.
- `field()` returns a stable storage buffer whose active prefix contains `rings() × vertices_per_ring()` consecutive `vec4<f32>` records: world XYZ and age in seconds. Ring endpoints wrap within their own ring. This is filament geometry, **not** a sampled velocity grid.
- `params()` returns the stable 64-byte uniform used by the sampler below. Both buffers retain their identity across updates, settings changes and resets, so consumer bind groups do not need rebuilding.
- `set_settings(settings)` validates before changing state. New parameters apply on the next `step`. Changing origin, ring radius or lifetime also reseeds the rings, preserving the drift clock. Other settings preserve their current shape.
- `step(device, encoder, dt)` accepts finite `dt` in `0..=1/30` seconds and checks device identity. `dt = 0` applies settings/reseeding without advancing motion. Pausing/clamping a frame delta is the caller's decision.
- `reset(encoder)` restores staggered rings using current settings and resets simulation time to zero. It does not reset consumer particles.

Encode simulation and consumers in order, and submit encoders in the same order. Multiple steps, settings changes and resets may share one encoder: uniform and seed uploads are encoder-ordered. Each step reads the stable field, writes scratch, then copies scratch back into the field. The copy is entirely on the GPU (1,152 bytes for six twelve-vertex rings); there is no per-frame CPU vertex upload or GPU readback. Consume an intermediate state before encoding the next step if it is needed.

## WGSL velocity sampling

Concatenate `belfast::filament::WGSL_FILAMENT` with your consumer shader. Declare these two variables at binding locations chosen by your pipeline:

```wgsl
@group(0) @binding(0) var<uniform> filament_params: FilamentParams;
@group(0) @binding(1) var<storage, read> filament_vertices: array<vec4<f32>>;

// Inside your particle compute entry point:
// let velocity = filament_velocity(particle_position);
```

Bind `filaments.params().gpu().as_entire_binding()` to the uniform and `filaments.field().gpu().as_entire_binding()` to the storage binding. The exported snippet supplies `FilamentParams` and `filament_velocity(position)`. Its helper names have the `filament_` prefix; it declares no bindings or entry point.

The result is induced velocity in world units per second. Filament updraft and horizontal drift translate the rings; they are not an extra uniform force on consumer particles. Add a particle-specific lift, drag or integration scheme in your own compute shader. The courtyard keeps its existing petal drag, quaternion rotation and shadow-volume respawning there.

## Settings and limits

| Setting       | Default   | Meaning                                            |
| ------------- | --------- | -------------------------------------------------- |
| `origin`      | `[0,0,0]` | Ring emission origin in world space                |
| `ring_radius` | `0.21`    | Nominal ring radius, in world units                |
| `core_radius` | `0.036`   | Segment smoothing radius, in world units           |
| `updraft`     | `0.18`    | Ring +Y drift, in world units/second               |
| `xz_drift`    | `0.06`    | Coherent, slowly turning horizontal drift per ring |
| `strength`    | `1.61`    | Multiplier on the induced velocity                 |
| `circulation` | `0.003`   | Vortex circulation coefficient                     |
| `lifetime`    | `6.0`     | Seconds before each ring respawns                  |

Radii and lifetime must be finite and positive; motion/strength/circulation must be finite and nonnegative. Initial ages span one lifetime, with the existing lateral stagger and vertical spacing of 0.12 world units per second of age. Respawns retain ±15% radius variation and a small vertical deformation. Rings fade in during their first 5% of life and fade out during the final sixth.

Induced speed is capped at 0.6 world units/second; ring advection is capped at 0.8. Sampling sums every segment, and self-advection is quadratic in total segment count. Keep ring counts/resolution small for large particle systems; no spatial acceleration is built in.

The regularized segment formula is adapted from David Li's `vortexspheres`. Its MIT notice is included in [sampling.wgsl](sampling.wgsl).
