struct Uniforms {
    view_projection: mat4x4<f32>,
    model: mat4x4<f32>,
    mode: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var volume: texture_3d<f32>;
@group(0) @binding(2) var volume_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) sample_pos: vec3<f32>,
    @location(1) alpha: f32,
};

const PLANE = array<vec2<f32>, 4>(
    vec2<f32>(-0.5, -0.5),
    vec2<f32>(0.5, -0.5),
    vec2<f32>(-0.5, 0.5),
    vec2<f32>(0.5, 0.5),
);
const SLICE = array<vec2<f32>, 4>(
    vec2<f32>(-1.0, -1.0),
    vec2<f32>(1.0, -1.0),
    vec2<f32>(-1.0, 1.0),
    vec2<f32>(1.0, 1.0),
);
const QUAD = array<u32, 6>(0u, 1u, 2u, 2u, 1u, 3u);

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // mode.y is how many full-frame slices precede the cutting plane.
    // They are ordered from z = -1 toward the camera, so blending is back to front.
    let slice_verts = u32(u.mode.y) * 6u;
    var output: VertexOutput;
    if (vertex_index < slice_verts) {
        let corner = SLICE[QUAD[vertex_index % 6u]];
        // Layer center. Keep this in sync with slice_z.
        let z = (f32(vertex_index / 6u) + 0.5) / u.mode.z * 2.0 - 1.0;
        let p = vec3<f32>(corner, z);
        output.position = u.view_projection * vec4<f32>(p, 1.0);
        output.sample_pos = p;
        output.alpha = 0.22;
    } else {
        let corner = PLANE[QUAD[vertex_index - slice_verts]];
        // Sample the rotated plane. Face-on mode only changes where it is drawn.
        let p = (u.model * vec4<f32>(corner, 0.0, 1.0)).xyz;
        let shown = select(p, vec3<f32>(corner, 0.0), u.mode.x > 0.5);
        output.position = u.view_projection * vec4<f32>(shown, 1.0);
        output.sample_pos = p;
        output.alpha = 1.0;
    }
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let p = input.sample_pos;
    // Image row 0 is the top, so flip Y. Keep this in sync with volume_uvw.
    let uvw = vec3<f32>(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5, p.z * 0.5 + 0.5);
    return vec4<f32>(textureSample(volume, volume_sampler, uvw).rgb, input.alpha);
}
