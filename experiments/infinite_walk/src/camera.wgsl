struct Camera {
    view_projection: mat4x4<f32>,
    // Projection X/Y scale, world-space point diameter, sRGB attachment flag.
    billboard: vec4<f32>,
    // Camera Z, distance behind camera, loop length, X drift per loop.
    walk: vec4<f32>,
    appearance: vec4<f32>,
    // Top center and intensity of the downward light cylinder.
    light: vec4<f32>,
    // Vertical range, outer radius, inner radius, ambient illumination.
    light_shape: vec4<f32>,
    light_view_projection: mat4x4<f32>,
    // Shadow map size, depth bias, strength, padding.
    shadow: vec4<f32>,
}
@group(0) @binding(0) var<uniform> camera: Camera;

fn wrapped_position(position: vec3<f32>) -> vec3<f32> {
    let loops = floor((camera.walk.x + camera.walk.y - position.z) / camera.walk.z);
    return position + vec3(-loops * camera.walk.w, 0.0, loops * camera.walk.z);
}

fn inside_light_cylinder(position: vec3<f32>) -> bool {
    let offset = position - camera.light.xyz;
    return camera.light.w > 0.0 && offset.y <= 0.0 && -offset.y < camera.light_shape.x
        && dot(offset.xz, offset.xz) < camera.light_shape.y * camera.light_shape.y;
}
