import { Camera } from "./Camera";
import { mat4 } from "gl-matrix";

class CameraPerspective extends Camera {
  constructor(mFov, mAspectRatio, mNear, mFar) {
    super();

    mat4.perspective(this._mtxProj, mFov, mAspectRatio, mNear, mFar);
  }
}

export { CameraPerspective };
