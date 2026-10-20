struct SimParams {
    min_cell: vec4<f32>, // lower world bound, grid cell size
    max_neighbor: vec4<f32>, // upper world bound, neighbor radius
    flock: vec4<f32>, // separation radius, separation/cohesion/alignment strength
    motion: vec4<f32>, // max speed, max acceleration, drag, soft boundary margin
    noise_phase: vec4<f32>, // curl scale, strength, animation speed, phase coupling
    clock: vec4<f32>, // dt, movement noise time, body radius, phase noise time
    controls: vec4<f32>, // movement rate, blinking rate, phase jitter, flash duration
    grid_count: vec4<u32>, // grid dimensions, particle count
    neighbor_view: array<vec4<f32>, 6>, // inward world planes, expanded by neighbor + sprite radius
    benchmark: vec4<u32>, // x: offscreen simplification enabled
};
@group(0) @binding(0) var<uniform> params: SimParams;
@group(0) @binding(1) var<storage, read> particles_in: array<Firefly>;
@group(0) @binding(2) var<storage, read_write> particles_out: array<Firefly>;
@group(0) @binding(3) var<storage, read_write> heads: array<atomic<u32>>;
@group(0) @binding(4) var<storage, read_write> links: array<u32>;

fn cell_of(position: vec3<f32>) -> vec3<i32> {
    return clamp(vec3<i32>(floor((position - params.min_cell.xyz) / params.min_cell.w)),
        vec3(0), vec3<i32>(params.grid_count.xyz) - vec3(1));
}
fn cell_index(cell: vec3<i32>) -> u32 {
    let c = vec3<u32>(cell);
    return c.x + params.grid_count.x * (c.y + params.grid_count.y * c.z);
}
fn limit_length(v: vec3<f32>, maximum: f32) -> vec3<f32> {
    return v * min(1., maximum / max(length(v), 0.00001));
}
fn needs_neighbor_search(position: vec3<f32>) -> bool {
    if params.benchmark.x == 0u { return true; }
    let point = vec4(position, 1.);
    for (var plane = 0u; plane < 6u; plane += 1u) {
        if dot(params.neighbor_view[plane], point) < 0. { return false; }
    }
    return true;
}

@compute @workgroup_size(64)
fn clear_grid(@builtin(global_invocation_id) id: vec3<u32>) {
    let count = params.grid_count.x * params.grid_count.y * params.grid_count.z;
    if id.x < count { atomicStore(&heads[id.x], 0u); }
}

@compute @workgroup_size(64)
fn build_grid(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= params.grid_count.w { return; }
    // Zero is the end sentinel; stored indices are offset by one. Every particle
    // has a link, so even crowded cells cannot overflow a fixed bucket capacity.
    // Keep offscreen particles here: onscreen neighbors must still see them.
    let cell = cell_index(cell_of(particles_in[id.x].position_phase.xyz));
    links[id.x] = atomicExchange(&heads[cell], id.x + 1u);
}

@compute @workgroup_size(64)
fn update_particles(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= params.grid_count.w { return; }
    let current = particles_in[id.x];
    let dt = params.clock.x;
    if dt <= 0. { particles_out[id.x] = current; return; }
    var position = current.position_phase.xyz;
    var velocity = current.velocity_frequency.xyz;
    var separation = vec3(0.);
    var center = vec3(0.);
    var heading = vec3(0.);
    var phase_difference = 0.;
    var neighbors = 0.;
    var close_neighbors = 0.;
    // Only skip neighbor interactions. Offscreen particles still integrate
    // motion/noise, enforce bounds and advance their independent flash cycle.
    if needs_neighbor_search(position) {
        let cell = cell_of(position);
        let radius_squared = params.max_neighbor.w * params.max_neighbor.w;
        for (var z = -1; z <= 1; z += 1) {
            for (var y = -1; y <= 1; y += 1) {
                for (var x = -1; x <= 1; x += 1) {
                    let c = cell + vec3(x, y, z);
                    if any(c < vec3(0)) || any(c >= vec3<i32>(params.grid_count.xyz)) { continue; }
                    var entry = atomicLoad(&heads[cell_index(c)]);
                    loop {
                        if entry == 0u { break; }
                        let other_id = entry - 1u;
                        entry = links[other_id];
                        if other_id == id.x { continue; }
                        let other = particles_in[other_id];
                        let away = position - other.position_phase.xyz;
                        let distance_squared = dot(away, away);
                        if distance_squared > radius_squared { continue; }
                        let distance = sqrt(distance_squared);
                        neighbors += 1.;
                        center += other.position_phase.xyz;
                        heading += other.velocity_frequency.xyz;
                        // Signed shortest circular distance. + means this neighbor
                        // is ahead (speed up); - means it is behind (slow down).
                        phase_difference += fract(other.position_phase.w - current.position_phase.w + 0.5) - 0.5;
                        if distance < params.flock.x {
                            var direction = away / max(distance, 0.00001);
                            if distance_squared < 0.00000001 {
                                // Equal-position pairs receive opposite, finite forces.
                                let pair = vec3<i32>(i32(min(id.x, other_id)), i32(max(id.x, other_id)), 17);
                                direction = normalize(petal_noise_hash(pair) + vec3(0.0001));
                                direction *= select(-1., 1., id.x < other_id);
                            }
                            separation += direction * (1. - distance / params.flock.x);
                            close_neighbors += 1.;
                        }
                    }
                }
            }
        }
    }
    var force = vec3(0.);
    var phase_rate = current.velocity_frequency.w;
    if neighbors > 0. {
        force += (center / neighbors - position) * params.flock.z;
        force += (heading / neighbors - velocity) * params.flock.w;
        phase_rate += params.noise_phase.w * phase_difference / neighbors;
    }
    // Stable individual offsets keep nearby fireflies from sharing one wind
    // direction or changing their random frequency drift at the same instant.
    let individual = petal_noise_hash(vec3<i32>(i32(id.x), 71, 43));
    let noise_time = params.clock.w * (0.25 + 0.10 * individual.y) + individual.z * 11.;
    let interval = i32(floor(noise_time));
    let fraction = fract(noise_time);
    let blend = fraction * fraction * (3. - 2. * fraction);
    let random_a = petal_noise_hash(vec3<i32>(i32(id.x), interval, 29)).x;
    let random_b = petal_noise_hash(vec3<i32>(i32(id.x), interval + 1, 29)).x;
    phase_rate += mix(random_a, random_b, blend) * params.controls.z;
    if close_neighbors > 0. {
        force += separation / close_neighbors * params.flock.y;
    }
    // Shift the existing curl sample for each particle; no additional curl
    // evaluation or neighbor search is needed for the individual wandering.
    let offset = individual * 13.
        + vec3(0.73, 0.41, -0.57) * params.clock.y * params.noise_phase.z * (0.7 + 0.3 * individual.x);
    force += curl_noise(position * params.noise_phase.x + offset) * params.noise_phase.y;
    let low = params.min_cell.xyz + vec3(params.clock.z);
    let high = params.max_neighbor.xyz - vec3(params.clock.z);
    // Smoothly turn inward before reaching a wall; clamp is only a safety net.
    let lower_pressure = clamp((low + vec3(params.motion.w) - position) / params.motion.w, vec3(0.), vec3(1.));
    let upper_pressure = clamp((position - high + vec3(params.motion.w)) / params.motion.w, vec3(0.), vec3(1.));
    force += (lower_pressure * lower_pressure - upper_pressure * upper_pressure) * 0.9;
    force = limit_length(force, params.motion.y);
    let movement_dt = dt * params.controls.x;
    let steps = max(1u, u32(ceil(movement_dt * 120.)));
    let h = movement_dt / f32(steps);
    for (var step = 0u; step < steps && movement_dt > 0.; step += 1u) {
        velocity += force * h;
        velocity *= exp(-params.motion.z * h);
        velocity = limit_length(velocity, params.motion.x);
        position += velocity * h;
        for (var axis = 0u; axis < 3u; axis += 1u) {
            if position[axis] < low[axis] {
                position[axis] = low[axis];
                velocity[axis] = abs(velocity[axis]) * 0.3;
            }
            if position[axis] > high[axis] {
                position[axis] = high[axis];
                velocity[axis] = -abs(velocity[axis]) * 0.3;
            }
        }
    }
    // The oscillator controls when a flash starts, never how long it stays lit.
    let frequency = clamp(phase_rate, 0.05, 0.5) * params.controls.y;
    let next_phase = current.position_phase.w + frequency * dt;
    let phase = fract(next_phase);
    // Hold each pulse's chosen duration until the next cycle, so adjusting the
    // control cannot stretch an active pulse or relight an expired one.
    var duration = current.flash.z;
    var flash_age = min(current.flash.x + dt, duration);
    if next_phase >= 1. && frequency > 0. {
        duration = params.controls.w;
        // Account for the part of this frame after the exact cycle crossing.
        flash_age = min(phase / frequency, duration);
    }
    var intensity = 0.;
    if flash_age < duration {
        let envelope = sin(3.14159265359 * flash_age / duration);
        intensity = envelope * envelope;
    }
    // Use unscaled seconds even when blinking is paused, so an active flash
    // completes normally instead of becoming a permanently illuminated particle.
    particles_out[id.x] = Firefly(
        vec4(position, phase),
        vec4(velocity, current.velocity_frequency.w),
        vec4(flash_age, intensity, duration, 0.)
    );
}
