struct SpawnList {
    count: u32,
    padding0: u32,
    padding1: u32,
    padding2: u32,
    points: array<SourcePoint>,
}
@group(0) @binding(1) var<storage, read> filament_vertices: array<vec4<f32>>;
@group(0) @binding(2) var<uniform> filament_params: FilamentParams;
@group(0) @binding(3) var<storage, read> particles_in: array<Particle>;
@group(0) @binding(4) var<storage, read_write> particles_out: array<Particle>;
@group(0) @binding(5) var<storage, read> spawns: SpawnList;
@group(0) @binding(6) var<uniform> motion: Motion;

@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= motion.counts.x { return; }
    var p = particles_in[id.x];
    let dt = motion.step.x;
    // Use the same copy of the forest as the spawn pass when the walk loops.
    let position = wrapped_position(p.position.xyz);
    let age = p.velocity.w + dt * select(motion.life.w, 1.0, inside_light_cylinder(position));
    if id.x >= motion.counts.y || p.life <= 0.0 || age >= p.life {
        if spawns.count == 0u {
            p.position.w = 0.0;
            p.life = 0.0;
        } else {
            let seed = filament_hash_u32(p.seed + id.x + 1u);
            let source = spawns.points[seed % spawns.count];
            p.position = vec4(source.position, 0.0);
            p.velocity = vec4(0.0);
            p.color = source.color;
            p.life = motion.life.x * (1.0 + (filament_random01(seed + 1u) * 2.0 - 1.0) * motion.life.y);
            p.seed = seed;
            p.force_scale = mix(0.5, 1.5, filament_random01(seed + 2u));
        }
        particles_out[id.x] = p;
        return;
    }

    let origin = vec3(camera.light.x, motion.life.z, camera.light.z);
    // Filaments stay in local space so following the light never reseeds their rings.
    let field = filament_velocity(position - origin + p.velocity.xyz * dt * 0.5);
    let target_velocity = (field + vec3(0.0, motion.step.z, 0.0)) * p.force_scale;
    let velocity = filament_limit_velocity(mix(p.velocity.xyz, target_velocity, 1.0 - exp(-motion.step.w * dt)), 1.2);
    let fade = smoothstep(0.0, min(0.3, p.life * 0.2), age)
        * (1.0 - smoothstep(p.life * 0.8, p.life, age));
    p.position = vec4(position + (p.velocity.xyz + velocity) * (dt * 0.5), motion.step.y * fade);
    p.velocity = vec4(velocity, age);
    particles_out[id.x] = p;
}
