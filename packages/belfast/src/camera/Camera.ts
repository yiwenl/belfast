import * as mat4 from "../math/mat4";
import type { Mat4, Vec3 } from "../math/types";

const DEFAULT_UP: Vec3 = [0, 1, 0];

export class Camera {
  protected readonly viewMatrix: Mat4;
  protected readonly projectionMatrix: Mat4;
  private readonly viewProjectionMatrix: Mat4;

  private position: Vec3 = [0, 0, 1];
  private target: Vec3 = [0, 0, 0];
  private up: Vec3 = DEFAULT_UP;

  constructor() {
    this.viewMatrix = mat4.create();
    this.projectionMatrix = mat4.create();
    this.viewProjectionMatrix = mat4.create();
    mat4.identity(this.projectionMatrix);
    this.lookAt(this.position, this.target);
  }

  lookAt(eye: Vec3, center: Vec3, up: Vec3 = DEFAULT_UP): this {
    this.position = [eye[0], eye[1], eye[2]];
    this.target = [center[0], center[1], center[2]];
    this.up = [up[0], up[1], up[2]];
    mat4.lookAt(this.viewMatrix, this.position, this.target, this.up);
    return this;
  }

  getViewMatrix(): Mat4 {
    return this.viewMatrix;
  }

  getProjectionMatrix(): Mat4 {
    return this.projectionMatrix;
  }

  getViewProjectionMatrix(out?: Mat4): Mat4 {
    const result = out ?? this.viewProjectionMatrix;
    return mat4.multiply(result, this.projectionMatrix, this.viewMatrix);
  }

  getPosition(): Vec3 {
    return [this.position[0], this.position[1], this.position[2]];
  }

  getLookAtTarget(): Vec3 {
    return [this.target[0], this.target[1], this.target[2]];
  }

  getFieldOfView(): number | undefined {
    return undefined;
  }

  protected updateProjection(): void {
    // Overridden by subclasses.
  }
}
