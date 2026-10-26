# Firefly projection interaction

**Goal:** Add opt-in nose-controlled camera movement for a large projection.

**Design:** Use the user's selected image-coordinate mapping. The image centre
maps to camera XY = (0, 0), with one scale control for both axes. Preserve the
Station's existing mirror transform, invert image Y, and keep camera Z at its
default 5 while looking at the forest centre. Smooth movement over roughly
0.6 seconds. Missing or low-confidence tracking holds the last position.
Default scale is 2. Either wrist above its same-side shoulder holds the existing
drag effect; lowering both hands releases it. Hand detection is independent of
nose visibility and uses image Y coordinates with valid, confident landmarks.
Lost tracking releases the effect. There is no movement detection or threshold control.
Exiting restores the previous orbital view. Station is already running locally
on port 8787. A same-origin WebSocket proxy supports both localhost and HTTPS LAN.

**Constraints:** Continue on the existing experiment branch. No tests, per user;
build WASM and LAN assets and inspect the running integration instead. No new
dependencies. Do not record skeleton streams. Do not commit unless requested.

- [x] Add Rust interaction mode and finite XY input; skip orbital updates while
      active, preserve normal sorting/cache invalidation and restore orbit on exit.
- [x] Add the mode button, scale slider, tracking status, validated image-nose
      mapping, reconnect and cleanup; coordinate with benchmark input locking.
- [x] Add the Vite dev/preview WebSocket proxy, document usage, rebuild and
      inspect the actual Station connection and UI without running test suites.

Review focus: stale/empty/invalid poses, mode toggles during pointer drags,
benchmark exclusivity, HTTPS proxy startup, socket/timer disposal.

Verification: release WASM, TypeScript and Vite build completed. Running page
shows active interaction, live nose tracking and a user-adjusted scale of 2.65.
An independent static review found no actionable bugs. No test suites were run.
