@vertex fn vs_instances(@location(0) position: vec3<f32>, @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>, @builtin(instance_index) index: u32) -> VertexOut {
    let petal = petals[index];
    let world = petal_world_position(position, petal);
    var out: VertexOut;
    out.clip = scene.view_projection * vec4(world, 1.);
    out.position = world;
    out.normal = rotate_petal(normal, petal.axis_speed.xyz, petal.velocity_angle.w);
    out.uv = uv;
    return out;
}
