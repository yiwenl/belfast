import { Camera } from "./Camera";
import * as mat4 from "../math/mat4";

export class OrthographicCamera extends Camera {
  private left = -1;
  private right = 1;
  private bottom = -1;
  private top = 1;
  private near = 0.1;
  private far = 100;

  constructor(left: number, right: number, bottom: number, top: number, near = 0.1, far = 100) {
    super();
    this.setOrthographic(left, right, bottom, top, near, far);
  }

  setOrthographic(
    left: number,
    right: number,
    bottom: number,
    top: number,
    near = 0.1,
    far = 100,
  ): this {
    this.left = left;
    this.right = right;
    this.bottom = bottom;
    this.top = top;
    this.near = near;
    this.far = far;
    mat4.ortho(this.getProjectionMatrix(), left, right, bottom, top, near, far);
    return this;
  }

  getFieldOfView(): undefined {
    return undefined;
  }

  protected override updateProjection(): void {
    this.setOrthographic(this.left, this.right, this.bottom, this.top, this.near, this.far);
  }
}
