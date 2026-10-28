struct Simulation { clock: vec4<f32>, motion: vec4<f32> }
@group(0) @binding(0) var<uniform> sim: Simulation;
@group(0) @binding(1) var<storage, read> particles_in: array<Particle>;
@group(0) @binding(2) var<storage, read_write> particles_out: array<Particle>;
@group(0) @binding(3) var<storage, read> fluid: array<vec4<f32>>;
fn fluid_cell(p: vec2<i32>) -> vec4<f32> {
    let n = i32(sim.clock.w);
    let c = clamp(p,vec2<i32>(0),vec2<i32>(n-1));
    return fluid[u32(c.y*n+c.x)];
}
fn sample_fluid(uv: vec2<f32>) -> vec4<f32> {
    let p = clamp(uv*sim.clock.w-0.5,vec2<f32>(0.0),vec2<f32>(sim.clock.w-1.0));
    let a = vec2<i32>(floor(p)); let f = fract(p);
    return mix(mix(fluid_cell(a),fluid_cell(a+vec2<i32>(1,0)),f.x),
        mix(fluid_cell(a+vec2<i32>(0,1)),fluid_cell(a+vec2<i32>(1,1)),f.x),f.y);
}
fn hash(p: vec3<f32>) -> f32 { return fract(sin(dot(p, vec3<f32>(127.1,311.7,74.7)))*43758.5453); }
fn noise(p: vec3<f32>) -> f32 {
    let i = floor(p); let f = fract(p); let u = f*f*(3.0-2.0*f);
    return mix(mix(mix(hash(i),hash(i+vec3<f32>(1,0,0)),u.x),mix(hash(i+vec3<f32>(0,1,0)),hash(i+vec3<f32>(1,1,0)),u.x),u.y),
        mix(mix(hash(i+vec3<f32>(0,0,1)),hash(i+vec3<f32>(1,0,1)),u.x),mix(hash(i+vec3<f32>(0,1,1)),hash(i+vec3<f32>(1,1,1)),u.x),u.y),u.z)*2.0-1.0;
}
@compute @workgroup_size(64) fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= u32(sim.clock.z) { return; }
    var p = particles_in[id.x];
    let dt = sim.clock.x;
    let flow = sample_fluid(p.position.xy/12.0+0.5);
    let density = mix(0.5,1.0,clamp(flow.w,0.0,1.0));
    let z = noise(p.position.xyz*0.75+vec3<f32>(sim.clock.y*0.5))*4.5*sim.motion.y;
    let force = vec3<f32>(flow.xy*15.36*sim.motion.x,z)*density*p.extra.z;
    let velocity = (p.velocity.xyz+force*dt)*pow(0.9,dt*60.0);
    p.position = vec4<f32>(p.position.xyz+velocity*dt,p.position.w-p.origin.w*dt);
    p.velocity = vec4<f32>(velocity,0.0);
    if p.position.w < 0.0 {
        p.position = vec4<f32>(p.origin.xyz,1.0);
    }
    particles_out[id.x] = p;
}
