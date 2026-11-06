@group(2) @binding(0) var scene_color: texture_2d<f32>;
@group(2) @binding(1) var scene_depth: texture_depth_2d;

fn unproject(ndc: vec2<f32>, depth: f32) -> vec3<f32> {
    let world = camera.inverse_view_projection * vec4(ndc, depth, 1.0);
    return world.xyz / world.w;
}

fn fog_hash(cell: vec3<i32>) -> f32 {
    let period = i32(camera.fog_noise_offset.w);
    let wrapped = cell % vec3(period);
    let c = vec3<u32>(select(wrapped, wrapped + vec3(period), wrapped < vec3(0)));
    let stride = u32(period);
    var hash = c.x + stride * c.y + stride * stride * c.z;
    hash = (hash ^ (hash >> 16u)) * 0x7feb352du;
    hash = (hash ^ (hash >> 15u)) * 0x846ca68bu;
    hash = hash ^ (hash >> 16u);
    return f32(hash >> 8u) / 16777216.0;
}

fn fog_noise(p: vec3<f32>) -> f32 {
    let cell = vec3<i32>(floor(p));
    let f = fract(p);
    let blend = f * f * (3.0 - 2.0 * f);
    let lower = mix(
        mix(fog_hash(cell), fog_hash(cell + vec3(1, 0, 0)), blend.x),
        mix(fog_hash(cell + vec3(0, 1, 0)), fog_hash(cell + vec3(1, 1, 0)), blend.x),
        blend.y,
    );
    let upper = mix(
        mix(fog_hash(cell + vec3(0, 0, 1)), fog_hash(cell + vec3(1, 0, 1)), blend.x),
        mix(fog_hash(cell + vec3(0, 1, 1)), fog_hash(cell + vec3(1, 1, 1)), blend.x),
        blend.y,
    );
    return mix(lower, upper, blend.z);
}

fn fog_noise_space(world: vec3<f32>) -> vec3<f32> {
    let path = vec2(camera.walk.w, -camera.walk.z);
    let path_length = length(path);
    let forward = path / path_length;
    let right = vec2(-forward.y, forward.x);
    // A whole number of periodic cells keeps the noise continuous when the camera rebases.
    return vec3(dot(world.xz, right), world.y, dot(world.xz, forward))
        * (camera.fog_noise_offset.w / path_length);
}

@fragment
fn fs_fog(in: ScreenVertex) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(in.position.xy);
    let surface = textureLoad(scene_color, pixel, 0);
    let ndc = in.uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0);
    let start = unproject(ndc, 0.0);
    let segment = unproject(ndc, textureLoad(scene_depth, pixel, 0)) - start;
    let distance = length(segment);
    if distance < 0.0001 { return surface; }
    let ray = segment / distance;

    // Clip the march to the light bounds and the first opaque surface on this ray.
    let origin = light_space(start);
    let beam_ray = (camera.light_view_projection * vec4(ray, 0.0)).xyz;
    let inv_ray = select(vec3(-1.0), vec3(1.0), beam_ray >= vec3(0.0)) / max(abs(beam_ray), vec3(0.000001));
    let a = (vec3(-1.0, -1.0, 0.0) - origin) * inv_ray;
    let b = (vec3(1.0) - origin) * inv_ray;
    let lo = min(a, b);
    let hi = max(a, b);
    let enter = max(0.0, max(lo.x, max(lo.y, lo.z)));
    let exit = min(distance, min(hi.x, min(hi.y, hi.z)));
    if exit <= enter { return surface; }

    // A stationary sub-step offset breaks up sampling bands without temporal flicker.
    let jitter = fract(52.9829189 * fract(dot(in.position.xy, vec2(0.06711056, 0.00583715))));
    let step_size = (exit - enter) / 32.0;
    let noise_origin = fog_noise_space(start) + camera.fog_noise_offset.xyz;
    let noise_ray = fog_noise_space(ray);
    var transmittance = 1.0;
    var scattering = 0.0;
    for (var i = 0u; i < 32u; i++) {
        let t = enter + (f32(i) + jitter) * step_size;
        let beam = origin + beam_ray * t;
        let edge = beam_edge(beam.xy);
        if edge <= 0.0 { continue; }
        let coords = shadow_coordinates(beam);
        let visibility = textureSampleCompareLevel(shadow_map, shadow_sampler, coords.xy, coords.z);
        let lit = light_at_depth(beam.z) * edge * mix(1.0, visibility, camera.shadow.z);
        // The porous forest floor is visible from both sides, so taper around its world-space plane.
        let floor_distance = abs((start.y - camera.forest_floor_y) + ray.y * t);
        let variation = mix(0.65, 1.35, fog_noise(noise_origin + noise_ray * t));
        let density = camera.shadow.w * exp2(-floor_distance * camera.inverse_fog_half_height) * variation;
        let step_transmittance = exp(-density * edge * step_size);
        // Isotropic single scattering, integrated front to back.
        scattering += transmittance * (1.0 - step_transmittance) * lit * 0.07957747;
        transmittance *= step_transmittance;
    }
    var color = surface.rgb;
    if camera.billboard.w < 0.5 { color = srgb_to_linear(color); }
    color = color * transmittance + vec3(scattering);
    if camera.billboard.w < 0.5 { color = linear_to_srgb(color); }
    return vec4(color, surface.a);
}
