struct SpawnList {
    count: atomic<u32>,
    padding0: u32,
    padding1: u32,
    padding2: u32,
    points: array<SourcePoint>,
}
struct ForestPoint {
    source: SourcePoint,
    copy_direction: vec4<f32>,
}
@group(0) @binding(1) var<storage, read> sources: array<ForestPoint>;
@group(0) @binding(2) var<storage, read_write> spawns: SpawnList;

fn add_spawn(position: vec3<f32>, color: u32) {
    if !inside_light_cylinder(light_space(position)) { return; }
    let index = atomicAdd(&spawns.count, 1u);
    spawns.points[index] = SourcePoint(position, color);
}

@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= arrayLength(&sources) { return; }
    let point = sources[id.x];
    add_spawn(wrapped_position(point.source.position), point.source.color);
    add_spawn(forest_position(point.source.position, point.copy_direction.xyz, true), point.source.color);
}
