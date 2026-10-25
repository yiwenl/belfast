struct Appearance {
    lime: vec4<f32>, // rgb, warm share
    yellow: vec4<f32>, // rgb, minimum peak
    amber: vec4<f32>, // rgb, maximum peak
};
@group(0) @binding(2) var<uniform> look: Appearance;
override peak_brightness: f32 = 1.8;

fn boosted_flash(intensity: f32, boost: f32) -> f32 {
    return mix(intensity, 1., boost);
}

fn firefly_emission(index: u32, intensity: f32) -> vec4<f32> {
    var bits = index * 747796405u + 2891336453u;
    bits = ((bits >> ((bits >> 28u) + 4u)) ^ bits) * 277803737u;
    bits = (bits >> 22u) ^ bits;
    let individual = vec2(f32(bits & 65535u), f32(bits >> 16u)) / 65535.;
    let yellow_green = mix(look.lime.rgb, look.yellow.rgb, individual.x);
    var warmth = 0.;
    if look.lime.a > 0. {
        warmth = smoothstep(1. - look.lime.a, 1., individual.x);
    }
    let brightness = peak_brightness * intensity * mix(look.yellow.a, look.amber.a, individual.y * individual.y);
    return vec4(mix(yellow_green, look.amber.rgb, warmth), brightness);
}
