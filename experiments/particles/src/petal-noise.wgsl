// Three independent value-noise channels form a vector potential A.
// Analytic derivatives of quintic interpolation give curl(A) using just eight
// lattice samples, without repeated finite-difference noise evaluations.
fn petal_noise_hash(cell: vec3<i32>) -> vec3<f32> {
    let c = bitcast<vec3<u32>>(cell);
    let seed = (c.x * 1597334677u) ^ (c.y * 3812015801u) ^ (c.z * 2798796415u);
    var h = vec3(seed) ^ vec3(0x68bc21ebu, 0x02e5be93u, 0x967a889bu);
    h = (h ^ (h >> vec3(16u))) * vec3(0x7feb352du);
    h = (h ^ (h >> vec3(15u))) * vec3(0x846ca68bu);
    h = h ^ (h >> vec3(16u));
    return vec3<f32>(h >> vec3(8u)) * (2. / 16777216.) - 1.;
}

fn curl_noise(p: vec3<f32>) -> vec3<f32> {
    let cell = vec3<i32>(floor(p));
    let f = fract(p);
    let u = f * f * f * (f * (f * 6. - 15.) + 10.);
    let du = 30. * f * f * (f - 1.) * (f - 1.);
    let a = petal_noise_hash(cell);
    let b = petal_noise_hash(cell + vec3(1, 0, 0));
    let c = petal_noise_hash(cell + vec3(0, 1, 0));
    let d = petal_noise_hash(cell + vec3(1, 1, 0));
    let e = petal_noise_hash(cell + vec3(0, 0, 1));
    let f1 = petal_noise_hash(cell + vec3(1, 0, 1));
    let g = petal_noise_hash(cell + vec3(0, 1, 1));
    let h = petal_noise_hash(cell + vec3(1, 1, 1));
    let dx = mix(mix(b-a, d-c, u.y), mix(f1-e, h-g, u.y), u.z) * du.x;
    let dy = mix(mix(c-a, d-b, u.x), mix(g-e, h-f1, u.x), u.z) * du.y;
    let dz = mix(mix(e-a, f1-b, u.x), mix(g-c, h-d, u.x), u.y) * du.z;
    // Leave magnitude intact: normalization would destroy the zero-divergence property.
    return vec3(dy.z - dz.y, dz.x - dx.z, dx.y - dy.x);
}
