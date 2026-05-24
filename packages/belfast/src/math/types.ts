/** 3D vector as a readonly tuple. */
export type Vec3 = readonly [number, number, number];

/** Mutable 3D vector tuple (e.g. orbital position written per axis). */
export type MutVec3 = [number, number, number];

/** Column-major 4×4 matrix (16 floats), compatible with WGSL `mat4x4`. */
export type Mat4 = Float32Array;
