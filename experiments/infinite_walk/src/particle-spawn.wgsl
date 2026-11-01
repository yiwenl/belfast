struct SpawnList {
    count: atomic<u32>,
    padding0: u32,
    padding1: u32,
    padding2: u32,
    points: array<SourcePoint>,
}
@group(0) @binding(1) var<storage, read> sources: array<SourcePoint>;
@group(0) @binding(2) var<storage, read_write> spawns: SpawnList;

@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= arrayLength(&sources) || camera.light.w <= 0.0 { return; }
    let source = sources[id.x];
    let position = wrapped_position(source.position);
    let offset = position - camera.light.xyz;
    let distance = length(offset);
    if distance >= camera.light_shape.x || -offset.y / max(distance, 0.0001) <= camera.light_shape.z {
        return;
    }
    let index = atomicAdd(&spawns.count, 1u);
    spawns.points[index] = SourcePoint(position, source.color);
}
