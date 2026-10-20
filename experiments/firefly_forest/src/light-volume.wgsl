@group(0) @binding(0) var<storage, read> particles: array<Firefly>;
@group(0) @binding(1) var<uniform> volume: LightVolumeParams;

struct Vertex {
    @builtin(position) position: vec4<f32>,
    @location(0) offset: vec3<f32>,
    @location(1) @interpolate(flat) emission: vec3<f32>,
};

@vertex fn vs_main(@builtin(vertex_index) vertex: u32, @builtin(instance_index) instance: u32) -> Vertex {
    // first_instance encodes the slice without separate uniform writes per pass.
    let index = instance % volume.dimensions_capacity.w;
    let slice = instance / volume.dimensions_capacity.w;
    let p = particles[index];
    let y = volume.min_radius.y + (f32(slice) + 0.5) / f32(volume.dimensions_capacity.z) * volume.extent_strength.y;
    let dy = y - p.position_phase.y;
    let radius = volume.min_radius.w;
    if p.flash.y <= 0. || abs(dy) >= radius {
        return Vertex(vec4(2., 2., 2., 1.), vec3(0.), vec3(0.));
    }
    let corners = array<vec2<f32>, 4>(vec2(-1., -1.), vec2(1., -1.), vec2(-1., 1.), vec2(1., 1.));
    let offset = corners[vertex] * sqrt(radius * radius - dy * dy);
    let xz = p.position_phase.xz + offset;
    let uv = (xz - volume.min_radius.xz) / volume.extent_strength.xz;
    let emission = firefly_emission(index, p.flash.y);
    return Vertex(vec4(uv.x * 2. - 1., 1. - uv.y * 2., 0., 1.), vec3(offset.x, dy, offset.y), emission.rgb * emission.a);
}

@fragment fn fs_main(in: Vertex) -> @location(0) vec4<f32> {
    let distance_squared = dot(in.offset, in.offset) / (volume.min_radius.w * volume.min_radius.w);
    let falloff = max(0., 1. - distance_squared);
    return vec4(in.emission * falloff * falloff, 0.);
}
