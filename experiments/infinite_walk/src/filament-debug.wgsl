@group(0) @binding(1) var<storage, read> filaments: array<vec4<f32>>;

struct DebugVertex {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32, @builtin(instance_index) ring: u32) -> DebugVertex {
    let vertex = (index / 2u + index % 2u) % VERTICES_PER_RING;
    let origin = camera.light.xyz;
    let point = filaments[ring * VERTICES_PER_RING + vertex].xyz + origin;
    let colors = array<vec3<f32>, 6>(
        vec3(0.0, 1.0, 1.0), vec3(1.0, 0.4, 0.0), vec3(1.0, 0.0, 1.0),
        vec3(0.2, 1.0, 0.0), vec3(1.0, 1.0, 0.0), vec3(0.4, 0.6, 1.0),
    );
    var out: DebugVertex;
    out.position = camera.view_projection * vec4(point, 1.0);
    out.color = colors[ring % 6u];
    return out;
}

@fragment
fn fs_main(in: DebugVertex) -> @location(0) vec4<f32> {
    return vec4(in.color, 1.0);
}
