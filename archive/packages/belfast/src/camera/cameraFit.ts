import { vec3, type ReadonlyVec3 } from "gl-matrix";
import type { OrthographicCamera } from "./OrthographicCamera";

export interface FitOrthographicCameraToSphereOptions {
  camera: OrthographicCamera;
  center: ReadonlyVec3;
  radius: number;
  eye: ReadonlyVec3;
  up?: ReadonlyVec3;
  padding?: number;
}

export function fitOrthographicCameraToSphere({
  camera,
  center,
  radius,
  eye,
  up = vec3.fromValues(0, 1, 0),
  padding = 0,
}: FitOrthographicCameraToSphereOptions): OrthographicCamera {
  const distance = vec3.distance(eye, center);
  const r = radius * (1 + padding);

  camera.lookAt(eye, center, up);
  camera.setOrthographic(-r, r, -r, r, Math.max(0.01, distance - r), distance + r);

  return camera;
}

export interface FitOrthographicCameraToBoundsOptions {
  camera: OrthographicCamera;
  points: ReadonlyVec3[];
  eye: ReadonlyVec3;
  target: ReadonlyVec3;
  up?: ReadonlyVec3;
  padding?: number;
}

export function fitOrthographicCameraToBounds({
  camera,
  points,
  eye,
  target,
  up = vec3.fromValues(0, 1, 0),
  padding = 0,
}: FitOrthographicCameraToBoundsOptions): OrthographicCamera {
  camera.lookAt(eye, target, up);
  const viewMatrix = camera.getViewMatrix();

  let minX = Infinity;
  let maxX = -Infinity;
  let minY = Infinity;
  let maxY = -Infinity;
  let minZ = Infinity;
  let maxZ = -Infinity;

  const temp = vec3.create();
  for (let i = 0; i < points.length; i++) {
    vec3.transformMat4(temp, points[i], viewMatrix);
    minX = Math.min(minX, temp[0]);
    maxX = Math.max(maxX, temp[0]);
    minY = Math.min(minY, temp[1]);
    maxY = Math.max(maxY, temp[1]);
    // Note: Z is negative in view space (looking down -Z)
    minZ = Math.min(minZ, temp[2]);
    maxZ = Math.max(maxZ, temp[2]);
  }

  // Calculate center and sizes to apply padding uniformly
  const width = (maxX - minX) * (1 + padding);
  const height = (maxY - minY) * (1 + padding);
  const depth = (maxZ - minZ) * (1 + padding);

  const centerX = (minX + maxX) / 2;
  const centerY = (minY + maxY) / 2;
  const centerZ = (minZ + maxZ) / 2;

  // View space Z points from camera into the scene negatively
  // We want near to be positive and far to be positive for orthoZO,
  // where Z is mapped from 0 to 1.
  // In gl-matrix orthoZO, near and far are distances from the camera.
  // Since camera looks down -Z, a point at z = -10 has distance 10.
  // So near distance is -maxZ, far distance is -minZ.
  const nearDist = Math.max(0.01, -centerZ - depth / 2);
  const farDist = -centerZ + depth / 2;

  camera.setOrthographic(
    centerX - width / 2,
    centerX + width / 2,
    centerY - height / 2,
    centerY + height / 2,
    nearDist,
    farDist,
  );

  return camera;
}
