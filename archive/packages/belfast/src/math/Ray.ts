import { vec3, type ReadonlyVec3 } from "gl-matrix";

/**
 * A ray defined by an origin and a direction.
 * Used for picking / hit-testing against triangles and spheres.
 */
export class Ray {
  readonly origin: vec3;
  readonly direction: vec3;

  // Scratch vectors — allocated once, reused across calls.
  private readonly _target = vec3.create();
  private readonly _edge1 = vec3.create();
  private readonly _edge2 = vec3.create();
  private readonly _normal = vec3.create();
  private readonly _diff = vec3.create();
  private readonly _a = vec3.create();
  private readonly _b = vec3.create();
  private readonly _c = vec3.create();

  constructor(origin: ReadonlyVec3, direction: ReadonlyVec3) {
    this.origin = vec3.clone(origin);
    this.direction = vec3.clone(direction);
  }

  /** Set origin and direction in-place. */
  set(origin: ReadonlyVec3, direction: ReadonlyVec3): this {
    vec3.copy(this.origin, origin);
    vec3.copy(this.direction, direction);
    return this;
  }

  /** Point along the ray at parameter `t`. */
  at(t: number, out?: vec3): vec3 {
    const result = out ?? this._target;
    vec3.copy(result, this.direction);
    vec3.scale(result, result, t);
    vec3.add(result, result, this.origin);
    return result;
  }

  /**
   * Möller–Trumbore ray–triangle intersection.
   * Returns the intersection point (newly allocated vec3) or `null`.
   */
  intersectTriangle(
    pa: ReadonlyVec3,
    pb: ReadonlyVec3,
    pc: ReadonlyVec3,
    backfaceCulling = true,
  ): vec3 | null {
    vec3.copy(this._a, pa);
    vec3.copy(this._b, pb);
    vec3.copy(this._c, pc);

    vec3.sub(this._edge1, this._b, this._a);
    vec3.sub(this._edge2, this._c, this._a);
    vec3.cross(this._normal, this._edge1, this._edge2);

    let DdN = vec3.dot(this.direction, this._normal);
    let sign: number;

    if (DdN > 0) {
      if (backfaceCulling) {
        return null;
      }
      sign = 1;
    } else if (DdN < 0) {
      sign = -1;
      DdN = -DdN;
    } else {
      return null;
    }

    vec3.sub(this._diff, this.origin, this._a);

    vec3.cross(this._edge2, this._diff, this._edge2);
    const DdQxE2 = sign * vec3.dot(this.direction, this._edge2);
    if (DdQxE2 < 0) {
      return null;
    }

    vec3.cross(this._edge1, this._edge1, this._diff);
    const DdE1xQ = sign * vec3.dot(this.direction, this._edge1);
    if (DdE1xQ < 0) {
      return null;
    }

    if (DdQxE2 + DdE1xQ > DdN) {
      return null;
    }

    const QdN = -sign * vec3.dot(this._diff, this._normal);
    if (QdN < 0) {
      return null;
    }

    const t = QdN / DdN;
    const hit = vec3.create();
    this.at(t, hit);
    return hit;
  }

  /** Ray–sphere intersection. Returns the closest hit point or `null`. */
  intersectSphere(center: ReadonlyVec3, radius: number): vec3 | null {
    const v1 = vec3.create();
    vec3.sub(v1, center, this.origin);
    const tca = vec3.dot(v1, this.direction);
    const d2 = vec3.dot(v1, v1) - tca * tca;
    const r2 = radius * radius;

    if (d2 > r2) {
      return null;
    }

    const thc = Math.sqrt(r2 - d2);
    const t0 = tca - thc;
    const t1 = tca + thc;

    if (t0 < 0 && t1 < 0) {
      return null;
    }

    const hit = vec3.create();
    if (t0 < 0) {
      this.at(t1, hit);
    } else {
      this.at(t0, hit);
    }
    return hit;
  }
}
