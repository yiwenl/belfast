import { Camera } from "./Camera";
import { mat4 } from "gl-matrix";

export class PerspectiveCamera extends Camera {
  private fov = Math.PI / 4;
  private aspect = 1;
  private near = 0.1;
  private far = 100;

  constructor(fov: number, aspect: number, near: number, far: number) {
    super();
    this.setPerspective(fov, aspect, near, far);
  }

  setPerspective(fov: number, aspect: number, near: number, far: number): this {
    this.fov = fov;
    this.aspect = aspect;
    this.near = near;
    this.far = far;
    mat4.perspectiveZO(this.getProjectionMatrix(), fov, aspect, near, far);
    return this;
  }

  setAspect(aspect: number): this {
    this.aspect = aspect;
    this.updateProjection();
    return this;
  }

  getFieldOfView(): number {
    return this.fov;
  }

  getAspect(): number {
    return this.aspect;
  }

  getNear(): number {
    return this.near;
  }

  getFar(): number {
    return this.far;
  }

  protected override updateProjection(): void {
    this.setPerspective(this.fov, this.aspect, this.near, this.far);
  }
}
