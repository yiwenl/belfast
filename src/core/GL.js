import EventDispatcher from "events";
import objectAssign from "object-assign";
import { checkWebGL2, getExtensions } from "../utils";
import exposeGLProperties from "../utils/exposeGLProperties";
import defaultGLParameters from "./defaultGLParameters";

let _idTable = 0;

function GLTool() {
  // PRIVATE PROPERTIES
  let _viewport = [0, 0, 0, 0];
  let _aspectRatio = 0;

  // PUBLIC PROPERTIES
  this.id = `WebGLContext${_idTable++}`;
  this.canvas;
  this.gl;
  this.width = 0;
  this.height = 0;

  this.webgl2 = checkWebGL2();

  // PUBLIC METHODS

  /**
   * Initialize the WebGL Context
   *
   * @param {undefined|Canvas|WebGLRenderingContext|WebGL2RenderingContext} mSource the source element
   */
  this.init = function(mSource, mParameters = {}) {
    const params = objectAssign({}, defaultGLParameters, mParameters);

    if (mSource === undefined) {
      const canvas = document.createElement("canvas");
      this.init(canvas, params);
      return;
    } else if (mSource instanceof HTMLCanvasElement) {
      this.canvas = mSource;
      let target = this.webgl2 ? "webgl2" : "webgl1";
      if (mParameters.webgl1) {
        // force using WebGL1
        target = "webgl1";
        this.webgl2 = false;
      }
      this.gl = mSource.getContext(target, params);
    } else {
      if (mSource instanceof WebGL2RenderingContext) {
        this.webgl2 = true;
        this.gl = mSource;
        this.canvas = mSource.canvas;
      } else if (mSource instanceof WebGLRenderingContext) {
        this.webgl2 = false;
        this.gl = mSource;
        this.canvas = mSource.canvas;
      } else {
        console.error(
          "The source has to be one of the following : Canvas, WebGLRenderingContext or WebGL2RenderingContext"
        );
      }
    }

    // Enable extensions
    this.extensions = getExtensions(this.gl);

    // Expose GL properties
    exposeGLProperties(this);

    // Set size
    this.setSize(this.canvas.width, this.canvas.height);

    // Set default blending to alpha blending
    this.enable(this.BLEND);
    this.enableAlphaBlending();
  };

  /**
   * Clear WebGL Context
   *
   * @param {number} r the red value
   * @param {number} g the green value
   * @param {number} b the blue value
   * @param {number} a the alpha value
   */
  this.clear = function(r = 0, g = 0, b = 0, a = 0) {
    const { gl } = this;
    gl.clearColor(r, g, b, a);
    gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
  };

  /**
   * Set WebGL size
   *
   * @param {number} mWidth the width
   * @param {number} mHeight the height
   */
  this.setSize = function(mWidth, mHeight) {
    this.width = Math.floor(mWidth);
    this.height = Math.floor(mHeight);
    this.canvas.width = this.width;
    this.canvas.height = this.height;
    _aspectRatio = this.width / this.height;

    this.viewport(0, 0, this.width, this.height);
  };

  /**
   * Set WebGL Viewport
   *
   * @param {number} x the x value
   * @param {number} y the y value
   * @param {number} w the width
   * @param {number} h the height
   */
  this.viewport = function(x, y, w, h) {
    _viewport = [x, y, w, h];
    this.gl.viewport(x, y, w, h);
  };

  /**
   * Set WebGL size
   *
   * @returns {vec4} the WebGL viewport
   */
  this.getViewport = function() {
    return _viewport;
  };

  /**
   * get WebGL canvas aspect ratio
   *
   * @returns {number} the aspect ratio
   */
  this.getAspectRatio = function() {
    return _aspectRatio;
  };

  /**
   * enable specific WebGL capabilities for this context.
   * @param {GLenum} the GLenum value of the capability
   */
  this.enable = function(mParameter) {
    this.gl.enable(mParameter);
  };

  /**
   * disable specific WebGL capabilities for this context.
   * @param {GLenum} the GLenum value of the capability
   */
  this.enable = function(mParameter) {
    this.gl.enable(mParameter);
  };

  /**
   * Set WebGL blending to Alpha blending
   *
   */
  this.enableAlphaBlending = function() {
    const { gl } = this;
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
  };

  /**
   * Set WebGL blending to Additive blending
   *
   */
  this.enableAdditiveBlending = function() {
    const { gl } = this;
    gl.blendFunc(gl.ONE, gl.ONE);
  };

  // PRIVATE METHODS

  const enableExtensions = () => {
    console.log("enable extensions");
  };
}

GLTool.prototype = Object.assign(Object.create(EventDispatcher.prototype), {
  constructor: GLTool,
});
const GL = new GLTool();
export { GL, GLTool };
