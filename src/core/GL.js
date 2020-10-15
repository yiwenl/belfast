import { mat4, mat3 } from "gl-matrix";
import { checkWebGL2, isMobile } from "../utils";
let gl;
class GLTool {
  constructor() {
    this._viewport = [0, 0, 0, 0];
    this._enabledVertexAttribute = [];
    this.identityMatrix = mat4.create();
    this._normalMatrix = mat3.create();
    this._inverseModelViewMatrix = mat3.create();
    this._modelMatrix = mat4.create();
    this._matrix = mat4.create();
    this._matrixStacks = [];
    this._lastMesh = null;
    this._useWebGL2 = false;
    this._hasArrayInstance = false;
    this._extArrayInstance = false;

    this.isMobile = isMobile;
  }

  init(mCanvas, mParameters = {}) {
    if (mCanvas === null || mCanvas === undefined) {
      console.error("Canvas not exist");
      return;
    }

    if (this.canvas !== undefined && this.canvas !== null) {
      this.destroy();
    }

    this.canvas = mCanvas;
    this.setSize(window.innerWidth, window.innerHeight);
    this.webgl2 = !!mParameters.webgl2 && checkWebGL2();
    console.log("this.webgl2", this.webgl2, mParameters);

    let ctx;
  }

  initWithGL(ctx) {
    if (!this.canvas) {
      this.canvas = ctx.canvas;
    }
    gl = this.gl = ctx;
  }

  setSize(mWidth, mHeight) {
    this._width = mWidth;
    this._height = mHeight;
    this.canvas.width = this._width;
    this.canvas.height = this._height;
    this._aspectRatio = this._width / this._height;

    if (gl) {
      this.viewport(0, 0, this._width, this._height);
    }
  }

  // DESTROY

  destroy() {
    if (this.canvas.parentNode) {
      try {
        this.canvas.parentNode.removeChild(this.canvas);
      } catch (e) {
        console.log("Error : ", e);
      }
    }

    this.canvas = null;
  }
}

const GL = new GLTool();
export { GL };
