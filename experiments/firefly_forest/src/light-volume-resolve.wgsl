@group(0) @binding(0) var slices: texture_2d_array<f32>;
@group(0) @binding(1) var volume: texture_storage_3d<rgba16float, write>;

@compute @workgroup_size(8, 8, 1)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id >= textureDimensions(volume)) { return; }
    textureStore(volume, id, textureLoad(slices, vec2<i32>(id.xy), i32(id.z), 0));
}
