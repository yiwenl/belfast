import { Camera } from "./Camera";
import { mat4 } from "gl-matrix";

class CameraOrtho extends Camera {
  constructor() {
    super();
  }

  ortho() {}
}

export { CameraOrtho };
