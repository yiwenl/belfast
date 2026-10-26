@group(0) @binding(0) var<storage, read> particles: array<Firefly>;
@group(0) @binding(1) var<storage, read_write> history: array<vec4<f32>>;
// Current slot, capacity/stride, active count, reset history.
@group(0) @binding(2) var<uniform> trail: vec4<u32>;

@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let index = id.x;
    if index >= trail.z { return; }
    // Capture the current individual boost too, shared by the whole live trail.
    let p = particles[index];
    let brightness = mix(p.flash.y, 1., smoothstep(0., 1., p.drag.x));
    let position = vec4(p.position_phase.xyz, brightness);
    if trail.w != 0u {
        for (var slot = 0u; slot < CHUNK_FRAMES; slot++) {
            history[slot * trail.y + index] = position;
        }
    } else {
        history[(trail.x % CHUNK_FRAMES) * trail.y + index] = position;
    }
}
