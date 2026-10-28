struct Globals { camera:mat4x4<f32>,light:mat4x4<f32>,appearance:vec4<f32> }
@group(0) @binding(0) var<uniform> globals:Globals;
@group(1) @binding(0) var<storage,read> particles:array<Particle>;
@group(2) @binding(0) var palette:texture_2d<f32>;
@group(2) @binding(1) var palette_sampler:sampler;
@group(2) @binding(2) var shadow:texture_depth_2d;
@group(2) @binding(3) var shadow_sampler:sampler_comparison;
struct VertexInput { @location(0) position:vec3<f32>,@location(1) normal:vec3<f32>,@builtin(instance_index) instance:u32 }
struct VertexOutput {
    @builtin(position) position:vec4<f32>,@location(0) normal:vec3<f32>,@location(1) uv:vec2<f32>,
    @location(2) shadow_coord:vec4<f32>,@location(3) appearance:vec2<f32>,
}
fn basis(v:vec3<f32>)->mat3x3<f32> {
    var x=vec3<f32>(1.0,0.0,0.0);
    if dot(v,v)>0.00000001 {x=normalize(v);}
    var reference=vec3<f32>(0.0,0.0,1.0);
    if abs(x.z)>0.99 {reference=vec3<f32>(0.0,1.0,0.0);}
    let y=normalize(cross(reference,x));
    return mat3x3<f32>(x,y,cross(x,y));
}
fn world_position(input:VertexInput)->vec3<f32> {
    let p=particles[input.instance];
    let life=1.0-smoothstep(0.4,0.5,abs(p.position.w-0.5));
    let speed=mix(0.3,1.0,smoothstep(0.0,18.0,length(p.velocity.xyz)));
    let local=input.position*vec3<f32>(p.extra.y,1.0,1.0)*p.extra.x*life*speed*globals.appearance.x;
    return p.position.xyz+basis(p.velocity.xyz)*local;
}
@vertex fn vs_main(input:VertexInput)->VertexOutput {
    let p=particles[input.instance];
    let world=world_position(input);
    var out:VertexOutput;
    out.position=globals.camera*vec4<f32>(world,1.0);
    out.normal=basis(p.velocity.xyz)*input.normal;
    out.uv=(p.origin.xy/12.0+0.5)*vec2<f32>(1.0,-1.0)+vec2<f32>(0.0,1.0);
    out.shadow_coord=globals.light*vec4<f32>(world,1.0);
    out.appearance=vec2<f32>(p.extra.x,p.extra.w);
    return out;
}
@vertex fn vs_shadow(input:VertexInput)->@builtin(position) vec4<f32> {
    return globals.light*vec4<f32>(world_position(input),1.0);
}
@fragment fn fs_main(input:VertexOutput)->@location(0) vec4<f32> {
    var color=pow(textureSample(palette,palette_sampler,input.uv).rgb+0.2,vec3<f32>(1.5));
    if input.appearance.x>3.0 {
        color=select(vec3<f32>(1.0,1.0,0.92),vec3<f32>(1.0,0.65,0.0),input.appearance.y<0.95)*1.2;
    }
    let diffuse=mix(0.85,1.0,max(dot(normalize(input.normal),normalize(vec3<f32>(0.1,0.6,0.6))),0.0));
    let visibility=sampleShadowPcf3x3(shadow,shadow_sampler,input.shadow_coord,globals.appearance.z,0.0007);
    color*=input.appearance.y*diffuse*mix(1.0,visibility,globals.appearance.y);
    return vec4<f32>(pow(max(color,vec3<f32>(0.0)),vec3<f32>(2.2)),1.0);
}
