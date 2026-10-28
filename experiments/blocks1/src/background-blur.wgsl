@group(0) @binding(0) var source:texture_2d<f32>;
@group(0) @binding(1) var source_sampler:sampler;
@group(0) @binding(2) var<uniform> params:vec4<f32>; // xy texel step, z downsample, w brightness
struct Output { @builtin(position) position:vec4<f32>,@location(0) uv:vec2<f32> }
@vertex fn vs_main(@builtin(vertex_index) id:u32)->Output {
    let uv=vec2<f32>(f32((id<<1u)&2u),f32(id&2u));
    var out:Output;out.position=vec4<f32>(uv*2.0-1.0,0.0,1.0);out.uv=vec2<f32>(uv.x,1.0-uv.y);return out;
}
fn linear_color(color:vec3<f32>)->vec3<f32> {
    return select(pow((color+0.055)/1.055,vec3<f32>(2.4)),color/12.92,color<=vec3<f32>(0.04045));
}
@fragment fn fs_main(input:Output)->@location(0) vec4<f32> {
    // Average every source texel when reducing the API's 512x512 image to 128x128.
    if params.z>0.5 {
        let base=vec2<i32>(input.position.xy)*4;
        let limit=vec2<i32>(textureDimensions(source))-vec2<i32>(1);
        var color=vec3<f32>(0.0);
        for(var y=0;y<4;y++) {
            for(var x=0;x<4;x++) {
                let pixel=clamp(base+vec2<i32>(x,y),vec2<i32>(0),limit);
                color+=linear_color(textureLoad(source,pixel,0).rgb);
            }
        }
        return vec4<f32>(color/16.0,1.0);
    }
    // Dense separable blur: sigma 16 here equals 64 pixels in the original image.
    var color=vec3<f32>(0.0);
    var total=0.0;
    for(var i=-48;i<=48;i++) {
        let offset=f32(i);
        let weight=exp(-offset*offset/512.0);
        color+=textureSampleLevel(source,source_sampler,input.uv+params.xy*offset,0.0).rgb*weight;
        total+=weight;
    }
    return vec4<f32>(color/total*params.w,1.0);
}
