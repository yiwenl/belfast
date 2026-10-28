@group(0) @binding(0) var source:texture_2d<f32>;
@group(0) @binding(1) var source_sampler:sampler;
struct Output { @builtin(position) position:vec4<f32>,@location(0) uv:vec2<f32> }
@vertex fn vs_main(@builtin(vertex_index) id:u32)->Output {
    let uv=vec2<f32>(f32((id<<1u)&2u),f32(id&2u));
    var out:Output;out.position=vec4<f32>(uv*2.0-1.0,0.0,1.0);out.uv=vec2<f32>(uv.x,1.0-uv.y);return out;
}
fn linear_color(color:vec3<f32>)->vec3<f32> {
    return select(pow((color+0.055)/1.055,vec3<f32>(2.4)),color/12.92,color<=vec3<f32>(0.04045));
}
@fragment fn fs_main(input:Output)->@location(0) vec4<f32> {
    // Only runs on image arrival, at 64x64: sigma is 1/8 of the source image.
    var color=vec3<f32>(0.0);
    var total=0.0;
    for(var y=-12;y<=12;y++) {
        for(var x=-12;x<=12;x++) {
            let delta=vec2<f32>(f32(x),f32(y));
            let weight=exp(-dot(delta,delta)/32.0);
            let sample=textureSampleLevel(source,source_sampler,input.uv+delta/32.0,0.0).rgb;
            color+=linear_color(sample)*weight;
            total+=weight;
        }
    }
    // Keep the backdrop two stops below the image so the blocks remain legible.
    return vec4<f32>(color/total*0.25,1.0);
}
