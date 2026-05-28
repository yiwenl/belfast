// 4× vec4 = 64 bytes per particle (matches vertex instance stride in main.ts)
struct Particle {
  position_size: vec4<f32>,
  velocity_speed: vec4<f32>,
  color: vec4<f32>,
  extra: vec4<f32>,
}

struct SimParams {
  time: f32,
  deltaTime: f32,
  maxRadius: f32,
  curlScale: f32,
  curlStrength: f32,
  damping: f32,
  boundaryStrength: f32,
  particleCount: f32,
}

@group(0) @binding(0) var<uniform> params: SimParams;
@group(0) @binding(1) var<storage, read_write> particles: array<Particle>;

// Classic GLSL simplex-style value noise (ported to WGSL)
fn mod289_v4(x: vec4<f32>) -> vec4<f32> {
  return x - floor(x * (1.0 / 289.0)) * 289.0;
}

fn perm(x: vec4<f32>) -> vec4<f32> {
  return mod289_v4(((x * 34.0) + 1.0) * x);
}

fn noise(p: vec3<f32>) -> f32 {
  let a = floor(p);
  var d = p - a;
  d = d * d * (3.0 - 2.0 * d);

  let b = vec4(a.x, a.x, a.y, a.y) + vec4(0.0, 1.0, 0.0, 1.0);
  let k1 = perm(vec4(b.x, b.y, b.x, b.y));
  let k2 = perm(vec4(k1.x, k1.y, k1.x, k1.y) + vec4(b.z, b.z, b.w, b.w));

  let c = k2 + vec4(a.z, a.z, a.z, a.z);
  let k3 = perm(c);
  let k4 = perm(c + vec4(1.0));

  let o1 = fract(k3 * (1.0 / 41.0));
  let o2 = fract(k4 * (1.0 / 41.0));

  let o3 = o2 * d.z + o1 * (1.0 - d.z);
  let o4 = vec2(o3.y, o3.w) * d.x + vec2(o3.x, o3.z) * (1.0 - d.x);

  return o4.y * d.y + o4.x * (1.0 - d.y);
}

// Remap value noise [0, 1] → [-1, 1] for curl field (GLSL snoise convention)
fn snoise(p: vec3<f32>) -> f32 {
  return noise(p) * 2.0 - 1.0;
}

fn snoiseVec3(x: vec3<f32>) -> vec3<f32> {
  let s = snoise(x);
  let s1 = snoise(vec3(x.y - 19.1, x.z + 33.4, x.x + 47.2));
  let s2 = snoise(vec3(x.z + 74.2, x.x - 124.5, x.y + 99.4));
  return vec3(s, s1, s2);
}

fn curlNoise(p: vec3<f32>) -> vec3<f32> {
  let e = 0.1;
  let dx = vec3(e, 0.0, 0.0);
  let dy = vec3(0.0, e, 0.0);
  let dz = vec3(0.0, 0.0, e);

  let p_x0 = snoiseVec3(p - dx);
  let p_x1 = snoiseVec3(p + dx);
  let p_y0 = snoiseVec3(p - dy);
  let p_y1 = snoiseVec3(p + dy);
  let p_z0 = snoiseVec3(p - dz);
  let p_z1 = snoiseVec3(p + dz);

  let x = p_y1.z - p_y0.z - p_z1.y + p_z0.y;
  let y = p_z1.x - p_z0.x - p_x1.z + p_x0.z;
  let z = p_x1.y - p_x0.y - p_y1.x + p_y0.x;

  let divisor = 1.0 / (2.0 * e);
  return normalize(vec3(x, y, z) * divisor);
}

@compute @workgroup_size(256)
fn cs_main(@builtin(global_invocation_id) globalId: vec3<u32>) {
  let index = globalId.x;
  if (f32(index) >= params.particleCount) {
    return;
  }

  var p = particles[index];
  let pos = p.position_size.xyz;
  let vel = p.velocity_speed.xyz;
  let particleSpeed = p.velocity_speed.w;

  let noiseStrength = mix(0.5, 2.0, p.extra.x);
  let t = params.time;
  // Oscillate on all axes — linear `t` on Y was drifting the field and flattening motion to XZ
  // let fieldOffset = vec3(sin(t * 0.5), sin(t * 0.37), cos(t * 0.3)) * 4.0;
  var fieldOffset = noise(pos.zxy + t * 2.0);
  fieldOffset = mix(1.0, 3.0, fieldOffset);
  let samplePos = pos * params.curlScale * fieldOffset * noiseStrength + t * 0.2;
  let force = curlNoise(samplePos) * params.curlStrength;

  var newVel = vel + force * params.deltaTime;
  newVel *= params.damping;

  let dist = length(pos);
  let boundaryStart = params.maxRadius * mix(0.5, 0.82, p.extra.y);
  if (dist > boundaryStart) {
    let t = smoothstep(boundaryStart, params.maxRadius, dist);
    let inward = -normalize(pos);
    newVel += inward * t * params.boundaryStrength * params.deltaTime;
  }
  var clampedPos = pos;
  if (dist > params.maxRadius) {
    clampedPos = normalize(pos) * params.maxRadius;
  }

  let newPos = clampedPos + newVel * params.deltaTime;

  p.position_size = vec4(newPos, p.position_size.w);
  p.velocity_speed = vec4(newVel, particleSpeed);
  particles[index] = p;
}
