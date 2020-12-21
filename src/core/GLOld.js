import { mat4, mat3 } from "gl-matrix";

import {
  checkWebGL2,
  isMobile,
  checkViewport,
  getWebGLContext,
} from "../utils";
import exposeAttributes from "../utils/exposeGLProperties";
import getAndApplyExtension from "../utils/getAndApplyExtension";
import ExtensionsList from "../utils/ExtensionsList";
import objectAssign from "object-assign";
import defaultGLParameters from "./defaultGLParameters";
import WebglNumber from "../utils/WebglNumber";

let _idTable = 0;

class GLTool {
  constructor() {
    this._id = `WebGLContext${_idTable++}`;
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

    // stats
    this.shaderCount = 0;
    this.meshCount = 0;
    this.bufferCount = 0;
    this.textureCount = 0;
    this.frameBufferCount = 0;

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
  initWithGL(gl) {
    if (!this.canvas) {
      this.canvas = ctx.canvas;
    }
    this.gl = gl;

    // extensions
    this.extensions = {};
    for (let i = 0; i < ExtensionsList.length; i++) {
      this.extensions[ExtensionsList[i]] = this.gl.getExtension(
        ExtensionsList[i]
      );
    }

    exposeAttributes(this);
    getAndApplyExtension(gl, "OES_vertex_array_object");
    getAndApplyExtension(gl, "ANGLE_instanced_arrays");
    getAndApplyExtension(gl, "WEBGL_draw_buffers");
    if (this.webgl2) {
      gl.getExtension("EXT_color_buffer_float");
    }

    // this.enable(this.DEPTH_TEST);
    // this.enable(this.CULL_FACE);
    this.enable(this.BLEND);
    this.enableAlphaBlending();
  }

  // enable gl feature
  enable(mParameter) {
    this.gl.enable(mParameter);
  }

  // disable gl feature
  disable(mParameter) {
    this.gl.disable(mParameter);
  }

  // BLEND MODES

  enableAlphaBlending() {
    const { gl } = this;
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
  }

  enableAdditiveBlending() {
    const { gl } = this;
    gl.blendFunc(gl.ONE, gl.ONE);
  }

  // set GL size
  setSize(mWidth, mHeight) {
    this._width = Math.floor(mWidth);
    this._height = Math.floor(mHeight);
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

  // show all extensions
  showExtensions() {
    console.log("Extensions : ", this.extensions);
    for (const ext in this.extensions) {
      if (this.extensions[ext]) {
        console.log(ext, ":", this.extensions[ext]);
      }
    }
  }

  // check if extensions is available
  checkExtension(mExtension) {
    return !!this.extensions[mExtension];
  }

  // get extension by name
  getExtension(mExtension) {
    return this.extensions[mExtension];
  }

  // set active shader
  useShader(mShader) {
    this.shader = mShader;
    this.shaderProgram = this.shader.shaderProgram;
  }

  // draw mesh
  draw(mMesh) {
    if (mMesh.length) {
      mMesh.forEach((m) => this.draw(m));
      return;
    }

    mMesh.bind(this.shaderProgram, this);
    const { drawType } = mMesh;
    const { gl } = this;

    if (mMesh.isInstanced) {
      // DRAWING
      gl.drawElementsInstanced(
        mMesh.drawType,
        mMesh.iBuffer.numItems,
        gl.UNSIGNED_SHORT,
        0,
        mMesh.numInstance
      );
    } else {
      if (drawType === gl.POINTS) {
        gl.drawArrays(drawType, 0, mMesh.vertexSize);
      } else {
        gl.drawElements(drawType, mMesh.iBuffer.numItems, gl.UNSIGNED_SHORT, 0);
      }
    }

    mMesh.unbind();
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
