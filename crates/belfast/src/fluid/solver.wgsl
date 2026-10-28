struct Splat { flow: vec4<f32>, shape: vec4<f32> }
struct Params { grid: vec4<f32>, counts: vec4<f32>, splats: array<Splat, 32> }
@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> field_in: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> field_out: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> pressure_in: array<f32>;
@group(0) @binding(4) var<storage, read_write> pressure_out: array<f32>;
@group(0) @binding(5) var<storage, read_write> divergence: array<f32>;
fn index(p: vec2<i32>) -> u32 {
    let n = i32(params.grid.x);
    // Match MIRRORED_REPEAT at texel centers, including negative coordinates.
    let period = vec2<i32>(2*n);
    let wrapped = ((p % period)+period) % period;
    let c = min(wrapped, period-vec2<i32>(1)-wrapped);
    return u32(c.y*n+c.x);
}
fn sample_field(p: vec2<f32>) -> vec4<f32> {
    let a = vec2<i32>(floor(p));
    let f = fract(p);
    return mix(mix(field_in[index(a)], field_in[index(a+vec2<i32>(1,0))], f.x),
        mix(field_in[index(a+vec2<i32>(0,1))], field_in[index(a+vec2<i32>(1,1))], f.x), f.y);
}
@compute @workgroup_size(8,8) fn advect(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= vec2<u32>(u32(params.grid.x))) { return; }
    let p = vec2<i32>(id.xy);
    let uv = (vec2<f32>(id.xy)+0.5)/params.grid.x;
    let velocity = field_in[index(p)].xy;
    var cell = sample_field(vec2<f32>(id.xy)-velocity*params.grid.y*params.grid.x);
    cell = cell * vec4<f32>(params.grid.zzz, params.grid.w);
    for (var i = 0u; i < u32(params.counts.x); i++) {
        let s = params.splats[i];
        let weight = 1.0-smoothstep(0.0,s.shape.x,distance(uv,s.flow.xy));
        cell += vec4<f32>(s.flow.zw,0.0,s.shape.y)*weight;
    }
    field_out[index(p)] = vec4<f32>(clamp(cell.xy,vec2<f32>(-20.0),vec2<f32>(20.0)), 0.0, clamp(cell.w,0.0,2.0));
}
fn velocity(p: vec2<i32>) -> vec2<f32> {
    // Texture mirroring changes sample coordinates, not the stored vector's sign.
    return field_in[index(p)].xy;
}
@compute @workgroup_size(8,8) fn divergence_main(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= vec2<u32>(u32(params.grid.x))) { return; }
    let p = vec2<i32>(id.xy);
    divergence[index(p)] = 0.5*params.grid.x*(velocity(p+vec2<i32>(1,0)).x-velocity(p-vec2<i32>(1,0)).x
        +velocity(p+vec2<i32>(0,1)).y-velocity(p-vec2<i32>(0,1)).y);
}
@compute @workgroup_size(8,8) fn jacobi(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= vec2<u32>(u32(params.grid.x))) { return; }
    let p = vec2<i32>(id.xy);
    let sum = pressure_in[index(p-vec2<i32>(1,0))]+pressure_in[index(p+vec2<i32>(1,0))]
        +pressure_in[index(p-vec2<i32>(0,1))]+pressure_in[index(p+vec2<i32>(0,1))];
    pressure_out[index(p)] = (sum-divergence[index(p)]/(params.grid.x*params.grid.x))*0.25;
}
@compute @workgroup_size(8,8) fn warm_pressure(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= vec2<u32>(u32(params.grid.x))) { return; }
    let cell = index(vec2<i32>(id.xy));
    pressure_out[cell] = pressure_in[cell] * params.counts.y;
}
@compute @workgroup_size(8,8) fn project(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= vec2<u32>(u32(params.grid.x))) { return; }
    let p = vec2<i32>(id.xy);
    // Preserve the source WebGL solver's gradient scale after converting to UV/second.
    let gradient = vec2<f32>(pressure_in[index(p+vec2<i32>(1,0))]-pressure_in[index(p-vec2<i32>(1,0))],
        pressure_in[index(p+vec2<i32>(0,1))]-pressure_in[index(p-vec2<i32>(0,1))])*params.grid.x;
    var cell = field_in[index(p)];
    cell = vec4<f32>(cell.xy-gradient,0.0,cell.w);
    field_out[index(p)] = cell;
}
