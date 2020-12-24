import { mat4 } from "gl-matrix";

class Camera {
  constructor() {
    this._mtxView = mat4.create();
    this._mtxProj = mat4.create();
  }

  /**
   * Update the view matrix with look At function
   *
   * @param {vec3} mEye the position of the camera
   * @param {vec3} mCenter the target of the camera looking at
   * @param {vec3} mUp the up vector
   */
  lookAt(mEye, mCenter, mUp = [0, 1, 0]) {
    mat4.lookAt(this._mtxView, mEye, mCenter, mUp);
  }

  /**
   * Set the camera from view & projection matrix
   *
   * @param {mat4} mView the view matrix
   * @param {mat4} mProj the projection matrix
   */
  setFromViewProjection(mView, mProj) {
    mat4.copy(this._mtxView, mView);
    mat4.copy(this._mtxProj, mProj);
  }

  /**
   * Update the view matrix of the camera
   *
   * @param {mat4} mMtx the view matrix
   */
  setViewMatrix(mMtx) {
    mat4.copy(this._mtxView, mMtx);
  }

  /**
   * Update the projection matrix of the camera
   *
   * @param {mat4} mMtx the projection matrix
   */
  setProjectionMatrix(mMtx) {
    mat4.copy(this._mtxProj, mMtx);
  }

  /**
   * Get view matrix from camera
   *
   * @returns {mat4} the view matrix
   */
  get viewMatrix() {
    return this._mtxView;
  }

  /**
   * Get view matrix from camera
   *
   * @returns {mat4} the view matrix
   */
  get view() {
    return this._mtxView;
  }

  /**
   * Get projection matrix from camera
   *
   * @returns {mat4} the projection matrix
   */
  get projectionMatrix() {
    return this._mtxProj;
  }

  /**
   * Get projection matrix from camera
   *
   * @returns {mat4} the projection matrix
   */
  get projection() {
    return this._mtxProj;
  }

  /**
   * Get the position of the camera
   *
   * @returns {vec3} the position of the camera
   */
  get position() {
    return [_mtxView[12], _mtxView[13], _mtxView[14]];
  }

  /**
   * Get the pointing direction of the camera
   *
   * @returns {vec3} the pointing direction of the camera
   */
  get direction() {
    // to be implemented
  }
}

export { Camera };
