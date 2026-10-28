struct Globals { camera:mat4x4<f32>,light:mat4x4<f32>,appearance:vec4<f32> }
@group(0) @binding(0) var background_from:texture_2d<f32>;
@group(0) @binding(1) var background_sampler:sampler;
@group(0) @binding(2) var background_to:texture_2d<f32>;
@group(0) @binding(3) var<uniform> globals:Globals;
struct Output { @builtin(position) position:vec4<f32>,@location(0) uv:vec2<f32> }
@vertex fn vs_main(@builtin(vertex_index) id:u32)->Output {
    let uv=vec2<f32>(f32((id<<1u)&2u),f32(id&2u));
    var out:Output;out.position=vec4<f32>(uv*2.0-1.0,0.999,1.0);out.uv=vec2<f32>(uv.x,1.0-uv.y);return out;
}
fn srgb_color(color:vec3<f32>)->vec3<f32> {
    return select(1.055*pow(color,vec3<f32>(1.0/2.4))-0.055,color*12.92,color<=vec3<f32>(0.0031308));
}
fn linear_color(color:vec3<f32>)->vec3<f32> {
    return select(pow((color+0.055)/1.055,vec3<f32>(2.4)),color/12.92,color<=vec3<f32>(0.04045));
}
fn pixel_noise(pixel:vec2<f32>)->f32 {
    let p=vec2<u32>(pixel);
    var h=p.x^(p.y*0x9e3779b9u);
    h=(h^(h>>16u))*0x7feb352du;
    h=(h^(h>>15u))*0x846ca68bu;
    h=h^(h>>16u);
    return f32(h>>8u)/16777216.0-0.5;
}
@fragment fn fs_main(input:Output)->@location(0) vec4<f32> {
    let color_from=textureSample(background_from,background_sampler,input.uv);
    let color_to=textureSample(background_to,background_sampler,input.uv);
    let color=mix(color_from,color_to,smoothstep(0.0,1.0,globals.appearance.w)).rgb;
    // Dither at the final display pixel, in sRGB code values, before 8-bit output.
    let encoded=clamp(srgb_color(max(color,vec3<f32>(0.0)))+pixel_noise(input.position.xy)/255.0,vec3<f32>(0.0),vec3<f32>(1.0));
    return vec4<f32>(linear_color(encoded),1.0);
}
