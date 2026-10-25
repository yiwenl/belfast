struct Camera {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    sprite: vec4<f32>, // Size, sRGB attachment, particle opacity, oldest visible age.
    effects: vec4<f32>, // Shared brightness boost, particle glow, reserved.
};
@group(0) @binding(0) var<storage, read> history: array<vec4<f32>>;
@group(0) @binding(1) var<uniform> camera: Camera;
@group(0) @binding(3) var<uniform> trail: vec4<u32>;
@group(0) @binding(4) var<storage, read> history1: array<vec4<f32>>;
@group(0) @binding(5) var<storage, read> history2: array<vec4<f32>>;
@group(0) @binding(6) var<storage, read> history3: array<vec4<f32>>;

struct Vertex {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) color: vec4<f32>,
};

fn history_sample(age: u32, index: u32) -> vec4<f32> {
    let slot = (trail.x + TRAIL_FRAMES - age) % TRAIL_FRAMES;
    let offset = (slot % CHUNK_FRAMES) * trail.y + index;
    switch slot / CHUNK_FRAMES {
        case 0u: { return history[offset]; }
        case 1u: { return history1[offset]; }
        case 2u: { return history2[offset]; }
        default: { return history3[offset]; }
    }
}

fn visible_position(age: u32, index: u32) -> vec3<f32> {
    // A fractional tail retracts smoothly; zero age pins every node to the live head.
    let sampled_age = min(f32(age), camera.sprite.w);
    let newer = u32(floor(sampled_age));
    let position = history_sample(newer, index).xyz;
    let fraction = fract(sampled_age);
    if fraction == 0. { return position; }
    return mix(position, history_sample(min(newer + 1u, TRAIL_FRAMES - 1u), index).xyz, fraction);
}

@vertex fn vs_main(@builtin(vertex_index) vertex: u32, @builtin(instance_index) index: u32) -> Vertex {
    // Share the particle's live flash and drag boost across the entire trail.
    let brightness = boosted_flash(history_sample(0u, index).w, camera.effects.x);
    if brightness <= 0. {
        // Unlit tubes must not write depth and hide illuminated tubes behind them.
        return Vertex(vec4(2., 2., 2., 1.), vec4(0.));
    }
    let age = vertex / 3u;
    let side = vertex % 3u;
    let center = visible_position(age, index);
    let newer = visible_position(max(age, 1u) - 1u, index);
    let older = visible_position(min(age + 1u, TRAIL_FRAMES - 1u), index);
    let direction = newer - older;
    var tangent = vec3(1., 0., 0.);
    if dot(direction, direction) > 0.0000000001 {
        tangent = normalize(direction);
    }
    let helper = select(vec3(0., 1., 0.), vec3(1., 0., 0.), abs(tangent.y) > 0.92);
    let normal = normalize(cross(tangent, helper));
    let binormal = cross(normal, tangent);
    let angle = f32(side) * 6.28318530718 / 3.;
    let radius = 0.003 * camera.sprite.x;
    let position = center + (normal * cos(angle) + binormal * sin(angle)) * radius;
    // Keep solid palette colors, with no HDR emission gain or glow.
    var color = firefly_emission(index, 1.).rgb;
    if camera.sprite.y > 0.5 {
        color = select(color / 12.92, pow((color + 0.055) / 1.055, vec3(2.4)), color > vec3(0.04045));
    }
    return Vertex(camera.projection * camera.view * vec4(position, 1.), vec4(color, brightness));
}

@fragment fn fs_main(in: Vertex) -> @location(0) vec4<f32> {
    if in.position.z > forest_depth(vec2<i32>(in.position.xy)) { discard; }
    return vec4(in.color.rgb, in.color.a * (1. - camera.sprite.z));
}
