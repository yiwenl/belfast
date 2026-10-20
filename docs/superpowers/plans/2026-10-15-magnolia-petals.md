# Magnolia petals

Implement the user's specified GLB instancing + GPU simulation in the existing Rust/Wasm experiment.

- Decode the supplied petal GLB and 1,435 tree-local emitter points in Rust. Share the actual branch placement matrix so changing tree placement also moves emitters. Preserve existing camera and lighting edits.
- Initialize 10,000 particles at emitters plus a small fixed random offset, with independent remaining/max lifetime, random unit rotation axis, spin speed, angle and size. Use the tree's uniform scale for petal geometry.
- Update two storage buffers with a compute pass before the visible pass. Apply a gentle diagonal upward acceleration and drag using seconds; reset position, velocity, age and angle when lifetime ends. Render the newly written buffer using the existing model materials/point light, one indexed instanced draw per primitive.
- Fetch assets alongside the courtyard; skip replacement assets in the original-tree comparison. Add a count to the overlay. Keep splat sorting independent.
- Verify asset alignment, invalid input handling, GPU motion/respawn/rotation and instanced drawing; run Rust tests, web tests and production build, then inspect the live scene and request a focused review.

Initial tuning: 6–12 second lives, about 0.03 scene-unit spawn jitter, randomized spin, wind toward +X/+Y/-Z. No new shadows, collision system or particle sorting is needed for these opaque petals.
