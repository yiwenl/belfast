struct Parameters {
    size: vec4<u32>,
    flow: vec4<f32>, // inertia, capacity, erosion, deposition
    water: vec4<f32>, // evaporation, gravity, initial water, lifetime
    soil: vec4<f32>, // settling rate, repose slope
};
struct Droplet {
    motion: vec4<f32>, // grid position, water, speed
    load: vec4<f32>, // direction, sediment, age
    exchange: vec4<f32>, // old position, requested erosion/deposition
    random: vec4<u32>, // random state, terminal flag
};
struct Exchange {
    erosion: atomic<u32>,
    deposition: atomic<u32>,
    accepted: f32,
    padding: f32,
};
@group(0) @binding(0) var<uniform> params: Parameters;
@group(0) @binding(1) var<storage, read_write> terrain: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> droplets: array<Droplet>;
@group(0) @binding(3) var<storage, read_write> exchanges: array<Exchange>;
@group(0) @binding(4) var<storage, read_write> soil_flow: array<vec4<f32>>;
const SCALE: f32 = 1000000.0;

fn random(state: ptr<function, u32>) -> f32 {
    *state ^= *state << 13u;
    *state ^= *state >> 17u;
    *state ^= *state << 5u;
    return f32(*state) / 4294967296.0;
}
fn height_gradient(p: vec2<f32>) -> vec3<f32> {
    let c = vec2<u32>(clamp(p, vec2(0.0), vec2(f32(params.size.x - 2u))));
    let f = fract(p);
    let i = c.x + c.y * params.size.x;
    let h = vec4(terrain[i].x, terrain[i + 1u].x, terrain[i + params.size.x].x, terrain[i + params.size.x + 1u].x);
    return vec3(mix(h.y - h.x, h.w - h.z, f.y), mix(h.z - h.x, h.w - h.y, f.x),
        mix(mix(h.x, h.y, f.x), mix(h.z, h.w, f.x), f.y));
}
fn weight(offset: vec2<i32>, fraction: vec2<f32>, radius: f32) -> f32 {
    return max(0.0, 1.0 - length(vec2<f32>(offset) - fraction) / radius);
}
fn totals(fraction: vec2<f32>) -> vec2<f32> {
    var total = vec2(0.0);
    for (var y = -3; y <= 3; y++) {
        for (var x = -3; x <= 3; x++) {
            let o = vec2(x, y);
            total += vec2(weight(o, fraction, 3.0), weight(o, fraction, 2.2));
        }
    }
    return total;
}
fn quantized(offset: vec2<i32>, fraction: vec2<f32>, normalization: vec2<f32>, amount: vec2<f32>) -> vec2<u32> {
    return vec2<u32>(amount * SCALE * vec2(weight(offset, fraction, 3.0), weight(offset, fraction, 2.2)) / normalization);
}

@compute @workgroup_size(64)
fn clear(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= params.size.x * params.size.x) { return; }
    atomicStore(&exchanges[id.x].erosion, 0u);
    atomicStore(&exchanges[id.x].deposition, 0u);
}

@compute @workgroup_size(64)
fn request(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= params.size.y) { return; }
    var d = droplets[id.x];
    var rng = d.random.x;
    if (d.load.w == 0.0) {
        let p = vec2(random(&rng), random(&rng)) * (f32(params.size.x) - 12.0) + 6.0;
        d.motion = vec4(p, params.water.z, 1.0);
        d.load = vec4(0.0, 0.0, 0.0, 1.0);
        d.random.y = 0u;
    }
    let old = d.motion.xy;
    var erosion = 0.0;
    var deposition = 0.0;
    if (d.random.y == 0u) {
        let hg = height_gradient(old);
        var direction = d.load.xy * params.flow.x - hg.xy * (1.0 - params.flow.x);
        let length_dir = length(direction);
        direction /= max(length_dir, 0.000001);
        let next = old + direction;
        let terminal = length_dir < 0.000001 || any(next < vec2(4.0)) || any(next > vec2(f32(params.size.x) - 5.0))
            || d.motion.z < 0.02 || d.load.w >= params.water.w;
        if (terminal) {
            d.random.y = 1u;
        } else {
            let change = height_gradient(next).z - hg.z;
            let capacity = max(0.002, max(-change, 0.0) * d.motion.w * d.motion.z * params.flow.y);
            if (change > 0.0) {
                deposition = min(change, d.load.z);
            } else if (d.load.z > capacity) {
                deposition = (d.load.z - capacity) * params.flow.w;
            } else {
                erosion = min(min((capacity - d.load.z) * params.flow.z, -change), 1.0);
            }
            d.motion = vec4(next, d.motion.z * (1.0 - params.water.x), sqrt(max(0.01, d.motion.w * d.motion.w - change * params.water.y)));
            d.load = vec4(direction, d.load.z, d.load.w + 1.0);
        }
    }
    if (d.random.y != 0u) { deposition = d.load.z; }
    deposition = min(deposition, 2.0);
    d.exchange = vec4(old, erosion, deposition);
    d.random.x = rng;
    droplets[id.x] = d;
    let cell = vec2<i32>(old);
    let fraction = fract(old);
    let normalization = totals(fraction);
    for (var y = -3; y <= 3; y++) {
        for (var x = -3; x <= 3; x++) {
            let offset = vec2(x, y);
            let q = quantized(offset, fraction, normalization, vec2(erosion, deposition));
            let index = u32(cell.x + x) + u32(cell.y + y) * params.size.x;
            atomicAdd(&exchanges[index].erosion, q.x);
            atomicAdd(&exchanges[index].deposition, q.y);
        }
    }
}

@compute @workgroup_size(64)
fn apply(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= params.size.x * params.size.x) { return; }
    let requested = f32(atomicLoad(&exchanges[id.x].erosion)) / SCALE;
    let deposit = f32(atomicLoad(&exchanges[id.x].deposition)) / SCALE;
    let previous = terrain[id.x];
    let eroded = min(requested, max(previous.x, 0.0));
    exchanges[id.x].accepted = select(0.0, eroded / max(requested, 0.0000001), requested > 0.0);
    terrain[id.x] = vec4(previous.x - eroded + deposit, previous.y, max(0.0, previous.z - eroded) + deposit, previous.w + eroded);
}

// Transport only loose deposited soil; the ridge substrate is never blurred.
@compute @workgroup_size(64)
fn settle_flow(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    let n = params.size.x;
    if (i >= n * n) { return; }
    let p = vec2<i32>(i32(i % n), i32(i / n));
    let offsets = array<vec2<i32>, 4>(vec2(-1, 0), vec2(1, 0), vec2(0, -1), vec2(0, 1));
    var flux = vec4(0.0);
    let h = terrain[i];
    for (var k = 0u; k < 4u; k++) {
        let q = p + offsets[k];
        if (all(q >= vec2(0)) && all(q < vec2(i32(n)))) {
            let neighbor = terrain[u32(q.x) + u32(q.y) * n].x;
            flux[k] = max(0.0, h.x - neighbor - params.soil.y) * params.soil.x;
        }
    }
    let total = dot(flux, vec4(1.0));
    soil_flow[i] = flux * min(1.0, h.z / max(total, 0.0000001));
}

@compute @workgroup_size(64)
fn settle_apply(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    let n = params.size.x;
    if (i >= n * n) { return; }
    let x = i % n;
    let y = i / n;
    var incoming = 0.0;
    if (x > 0u) { incoming += soil_flow[i - 1u].y; }
    if (x + 1u < n) { incoming += soil_flow[i + 1u].x; }
    if (y > 0u) { incoming += soil_flow[i - n].w; }
    if (y + 1u < n) { incoming += soil_flow[i + n].z; }
    let change = incoming - dot(soil_flow[i], vec4(1.0));
    terrain[i].x += change;
    terrain[i].z = max(0.0, terrain[i].z + change);
}

@compute @workgroup_size(64)
fn commit(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= params.size.y) { return; }
    var d = droplets[id.x];
    let cell = vec2<i32>(d.exchange.xy);
    let fraction = fract(d.exchange.xy);
    let normalization = totals(fraction);
    var actual = vec2(0.0);
    for (var y = -3; y <= 3; y++) {
        for (var x = -3; x <= 3; x++) {
            let q = quantized(vec2(x, y), fraction, normalization, d.exchange.zw);
            let index = u32(cell.x + x) + u32(cell.y + y) * params.size.x;
            actual += vec2(f32(q.x) * exchanges[index].accepted, f32(q.y)) / SCALE;
        }
    }
    d.load.z = max(0.0, d.load.z + actual.x - actual.y);
    // Quantization leaves at most one millionth per brush tap on a terminal droplet.
    if (d.random.y != 0u && d.load.z < 0.00005) { d.load.w = 0.0; }
    droplets[id.x] = d;
}
