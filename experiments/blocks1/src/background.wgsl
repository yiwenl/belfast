@group(0) @binding(0) var background:texture_2d<f32>;
@group(0) @binding(1) var background_sampler:sampler;
struct Output { @builtin(position) position:vec4<f32>,@location(0) uv:vec2<f32> }
@vertex fn vs_main(@builtin(vertex_index) id:u32)->Output {
    let uv=vec2<f32>(f32((id<<1u)&2u),f32(id&2u));
    var out:Output;out.position=vec4<f32>(uv*2.0-1.0,0.999,1.0);out.uv=vec2<f32>(uv.x,1.0-uv.y);return out;
}
@fragment fn fs_main(input:Output)->@location(0) vec4<f32> {return textureSample(background,background_sampler,input.uv);}
