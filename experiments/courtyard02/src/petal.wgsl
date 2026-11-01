struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) petal_t: f32,
    @location(2) world_position: vec3<f32>,
};

@vertex fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @builtin(instance_index) index: u32,
) -> VertexOut {
    let petal = petals[index];
    let world_position = petal_world_position(position, petal);
    var out: VertexOut;
    out.position = scene.view_projection * vec4<f32>(world_position, 1.0);
    out.normal = rotate_by_quaternion(normal, petal.rotation);
    out.petal_t = uv.y;
    out.world_position = world_position;
    return out;
}

@fragment fn fs_main(in: VertexOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    let normal = select(-normalize(in.normal), normalize(in.normal), front);
    let light = shadow.light_direction.xyz;
    let diffuse = max(dot(normal, light), 0.0);
    let transmitted = max(dot(-normal, light), 0.0);
    let facing_light = select(-normal, normal, dot(normal, light) >= 0.0);
    let visibility = shadow_visibility(in.world_position + facing_light * shadow.params.w);
    // Evaluate from UVs per fragment so the low-poly mesh cannot stretch the tint transition.
    let albedo = mix(scene.base_color.rgb, scene.body_color.rgb, smoothstep(0.0, 0.5, in.petal_t));
    // Shadow all lighting, including skylight, so full occlusion can reach black.
    var color = albedo * (0.48 + 0.48 * diffuse + 0.18 * transmitted) * visibility;
    if (scene.options.x < 0.5) {
        color = select(12.92 * color, 1.055 * pow(color, vec3<f32>(1.0 / 2.4)) - 0.055,
            color > vec3<f32>(0.0031308));
    }
    return vec4<f32>(color, 1.0);
}
