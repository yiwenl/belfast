@group(0) @binding(2) var env_irradiance: texture_2d<f32>;
@group(0) @binding(3) var env_specular: texture_2d<f32>;
@group(0) @binding(4) var env_sampler: sampler;

fn srgb_to_linear(color: vec3<f32>) -> vec3<f32> {
    return select(pow((color + 0.055) / 1.055, vec3(2.4)), color / 12.92, color <= vec3(0.04045));
}
fn linear_to_srgb(color: vec3<f32>) -> vec3<f32> {
    return select(1.055 * pow(color, vec3(1.0 / 2.4)) - 0.055, color * 12.92, color <= vec3(0.0031308));
}
fn tone_map(color: vec3<f32>) -> vec3<f32> {
    let x = max(color, vec3(0.0));
    // ACES fitted curve keeps HDR highlights within the SDR canvas range.
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), vec3(0.0), vec3(1.0));
}
fn hdr_uv(world_dir: vec3<f32>) -> vec2<f32> {
    let d = normalize(vec3(
        dot(view.env_x.xyz, world_dir),
        dot(view.env_y.xyz, world_dir),
        dot(view.env_z.xyz, world_dir),
    ));
    return vec2(
        atan2(d.z, d.x) * 0.15915494 + 0.5,
        acos(clamp(d.y, -1.0, 1.0)) * 0.31830989,
    );
}
fn pbr(normal: vec3<f32>, position: vec3<f32>, shadow_position: vec3<f32>, light_vec: vec3<f32>, cone: f32) -> vec3<f32> {
    let light_dir = normalize(light_vec);
    let shadow = terrain_shadow(shadow_position);
    let env_gain = view.light_gain.w;
    // The HDR sun was split out for shadows; Environment must add it back independently of Spot.
    let sun_cosine = max(dot(normal, view.light.xyz), 0.0);
    let sun = sun_cosine * (env_gain + view.light_gain.x);
    let spot = max(dot(normal, light_dir), 0.0) * cone;
    let direct = (sun + spot) * shadow;
    let env = textureSampleLevel(env_irradiance, env_sampler, hdr_uv(normal), 0.0).rgb * env_gain;
    let view_dir = normalize(view.eye.xyz - position * view.eye.w);
    let ndotv = max(dot(normal, view_dir), 0.001);
    let reflection = reflect(-view_dir, normal);
    let albedo = srgb_to_linear(view.mesh_color.rgb);
    let fresnel_base = mix(vec3(0.04), albedo, view.surface.x);
    let roughness = clamp(view.light_gain.z, 0.0, 1.0);
    // The unfiltered grazing Fresnel made even fully rough dark surfaces turn glossy white.
    let fresnel_grazing = max(vec3(1.0 - roughness), fresnel_base);
    let fresnel = fresnel_base + (fresnel_grazing - fresnel_base) * pow(1.0 - ndotv, 5.0);
    // The stored specular is the sharpest prefilter. Roughness broadens it without erasing it.
    let gloss = 1.0 - roughness;
    let sharp = textureSampleLevel(env_specular, env_sampler, hdr_uv(reflection), 0.0).rgb * env_gain;
    let sun_highlight = smoothstep(0.9, 0.99, max(dot(reflection, view.light.xyz), 0.0));
    let rough_env = env + view.sun_color.rgb * env_gain * sun_cosine * shadow;
    let specular = mix(rough_env, sharp * mix(1.0, shadow, sun_highlight), gloss) * fresnel;
    let diffuse = (1.0 - view.surface.x) * (1.0 - fresnel) * (env + direct * view.sun_color.rgb);
    return albedo * diffuse + specular;
}
fn surface_pbr(p: vec2<f32>, normal: vec3<f32>) -> vec3<f32> {
    // Shade the mesh position even when the ink layer is lifted above it.
    let position = mesh_position(p);
    let light_to_fragment = normalize(position - view.light.xyz * view.light.w);
    let angle = acos(clamp(dot(-view.light.xyz, light_to_fragment), -1.0, 1.0));
    let cone = pow(max(0.0, 1.0 - angle / view.material.x), view.material.y) * view.light_gain.y;
    return pbr(normal, position, world_position(p), -light_to_fragment, cone);
}
