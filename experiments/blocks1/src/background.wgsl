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
@fragment fn fs_main(input:Output)->@location(0) vec4<f32> {
    let color_from=textureSample(background_from,background_sampler,input.uv);
    let color_to=textureSample(background_to,background_sampler,input.uv);
    return mix(color_from,color_to,smoothstep(0.0,1.0,globals.appearance.w));
}
