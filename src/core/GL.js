import { mat4, mat3 } from "gl-matrix";
import { nanoid } from "nanoid";

import {
  checkWebGL2,
  isMobile,
  checkViewport,
  getWebGLContext,
} from "../utils";
import objectAssign from "object-assign";
import defaultGLParameters from "./defaultGLParameters";

let _idTable;

class GLTool {
  constructor() {
    this._id = nanoid();
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

  // initialize GL from canvas
  init(mSource, mParameters = {}) {
    if (mSource === null || mSource === undefined) {
      console.error("Canvas not exist");
      return;
    }

    if (mSource instanceof WebGLRenderingContext) {
      this.initWithGL(mSource);
    } else if (mSource instanceof WebGL2RenderingContext) {
      this.webgl2 = true;
      this.initWithGL(mSource);
    }

    if (this.canvas !== undefined && this.canvas !== null) {
      this.destroy();
    }

    this.canvas = mSource;
    this.webgl2 = !!mParameters.webgl2 && checkWebGL2();
    const params = objectAssign({}, defaultGLParameters, mParameters);
    const ctx = getWebGLContext(this.canvas, params, this.webgl2);

    this.initWithGL(ctx);
    this.setSize(window.innerWidth, window.innerHeight);
  }

  // initialize GL from WebGLContext
  initWithGL(ctx) {
    if (!this.canvas) {
      this.canvas = ctx.canvas;
    }
    this.gl = ctx;
  }

  // set GL size
  setSize(mWidth, mHeight) {
    this._width = mWidth;
    this._height = mHeight;
    this.canvas.width = this._width;
    this.canvas.height = this._height;
    this._aspectRatio = this._width / this._height;

    if (this.gl) {
      this.viewport(0, 0, this._width, this._height);
    }
  }

  // clear the GL context
  clear(r, g, b, a) {
    const { gl } = this;
    gl.clearColor(r, g, b, a);
    gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
  }

  // set GL Viewport
  viewport(x, y, w, h) {
    if (checkViewport(this._viewport, w, y, w, h)) {
      this.gl.viewport(x, y, w, h);
      this._viewport = [x, y, w, h];
    }
  }

  // getter & setters

  get id() {
    return this._id;
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
export { GLTool };
