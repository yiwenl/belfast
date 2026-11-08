struct Camera {
    view_projection: mat4x4<f32>,
    // Projection X/Y scale, world-space point diameter, sRGB attachment flag.
    billboard: vec4<f32>,
    // Camera Z, distance behind camera, loop length, X drift per loop.
    walk: vec4<f32>,
    appearance: vec2<f32>, // white override, copy offset per axis
    forest_floor_y: f32,
    inverse_fog_half_height: f32,
    // Beam center at filament ground height, and light intensity.
    light: vec4<f32>,
    // Axial range, outer radius, inner radius, ambient illumination.
    light_shape: vec4<f32>,
    light_view_projection: mat4x4<f32>,
    // Shadow map size, depth bias, strength, fog density per world unit.
    shadow: vec4<f32>,
    inverse_view_projection: mat4x4<f32>,
    fog_noise_offset: vec4<f32>, // periodic wind displacement in noise cells, cells per lap
    backdrop_y_offset: f32,
    backdrop_point_size: f32,
}
@group(0) @binding(0) var<uniform> camera: Camera;

fn wrapped_position(position: vec3<f32>) -> vec3<f32> {
    let loops = floor((camera.walk.x + camera.walk.y - position.z) / camera.walk.z);
    return position + vec3(-loops * camera.walk.w, 0.0, loops * camera.walk.z);
}

fn forest_position(position: vec3<f32>, direction: vec3<f32>, is_copy: bool) -> vec3<f32> {
    return wrapped_position(position + direction * select(0.0, camera.appearance.y, is_copy));
}

// The orthographic light projection maps radius to one and axial depth to [0, 1].
fn light_space(world: vec3<f32>) -> vec3<f32> {
    return (camera.light_view_projection * vec4(world, 1.0)).xyz;
}

fn inside_light_cylinder(beam: vec3<f32>) -> bool {
    return camera.light.w > 0.0 && beam.z >= 0.0 && beam.z < 1.0 && dot(beam.xy, beam.xy) < 1.0;
}
