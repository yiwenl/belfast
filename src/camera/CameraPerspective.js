import { Camera } from "./Camera";
import { mat4 } from "gl-matrix";

class CameraPerspective extends Camera {
  constructor(mFov, mAspectRatio, mNear, mFar) {
    super();
    this._fov = 0;
    this._ratio = 0;
    this.setPerspective(mFov, mAspectRatio, mNear, mFar);
  }

  /**
   * Update the projection matrix with perspective function
   *
   * @param {float} mFov the field of view
   * @param {float} mAspectRatio the aspect ratio
   * @param {float} mNear the near clip plane distance
   * @param {float} mFar the far clip plane distance
   */
  setPerspective(mFov, mAspectRatio, mNear, mFar) {
    mat4.perspective(this._mtxProj, mFov, mAspectRatio, mNear, mFar);
    this._near = mNear;
    this._far = mFar;
    this._fov = mFov;
    this._ratio = mAspectRatio;
  }

  /**
   * Set the aspect ratio of the camera
   *
   * @param {float} mAspectRatio the aspect ratio
   */
  setAspectRatio(mAspectRatio) {
    this._ratio = mAspectRatio;
    this._updateMatrices();
  }

  /**
   * Update the matrices after resetting the near or far clip plane
   *
   */
  _updateMatrices() {
    this.setPerspective(this._fov, this._ratio, this._near, this._far);
  }
}

export { CameraPerspective };
