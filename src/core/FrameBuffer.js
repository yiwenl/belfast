import { GL } from "./GL";
import { GLTexture } from "./GLTexture";
import { WebGLNumber } from "../utils/WebGLNumber";
import { WebGLConst } from "../utils/WebGLConst";

class FrameBuffer {
  constructor(mWidth, mHeight, mParameters = {}, mNumTargets = 1) {
    this._width = mWidth;
    this._height = mHeight;
    this._parameters = mParameters;
    this._numTargets = mNumTargets;

    this.frameBuffer;
    this._textures = [];

    this._initTextures();
  }

  /**
   * Bind the frame buffer
   *
   * @param {GL} mGL the GLTool instance
   * @param {boolean} mAutoSetViewport automatically set the viewport to framebuffer's viewport
   */
  bind(mGL, mAutoSetViewport = true) {
    if (mGL !== undefined && this.GL !== undefined && mGL !== this.GL) {
      console.error(
        "this frame buffer has been bind to a different WebGL Rendering Context",
        this.GL.id
      );
      return;
    }

    this.GL = mGL || GL;
    const { gl } = this.GL;

    if (this._numTargets > 1 && !this.GL.multiRenderTargetSupport) {
      console.error(
        `This browser doesn't support multi render targets : WEBGL_draw_buffers`
      );
    }

    if (!this.frameBuffer) {
      this._initFrameBuffer();
    }

    if (mAutoSetViewport) {
      this.GL.viewport(0, 0, this._width, this._height);
    }
    gl.bindFramebuffer(gl.FRAMEBUFFER, this.frameBuffer);
  }

  /**
   * Unbind the frame buffer
   *
   * @param {boolean} mAutoSetViewport automatically set the viewport back to GL's viewport
   */
  unbind(mAutoSetViewport = true) {
    if (mAutoSetViewport) {
      this.GL.viewport(0, 0, this.GL.width, this.GL.height);
    }
    const { gl } = this.GL;
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
  }

  /**
   * Initialize the framebuffer
   *
   */
  _initFrameBuffer() {
    const { gl } = this.GL;
    this.frameBuffer = gl.createFramebuffer();
    gl.bindFramebuffer(gl.FRAMEBUFFER, this.frameBuffer);
    this.GL.frameBufferCount++;

    const target = this.GL.webgl2 ? gl.DRAW_FRAMEBUFFER : gl.FRAMEBUFFER;

    // init textures;
    this._textures.forEach((t) => {
      t.createTexture(this.GL);
    });
    // this.glDepthTexture.createTexture(this.GL);

    const buffers = [];
    for (let i = 0; i < this._numTargets; i++) {
      gl.framebufferTexture2D(
        target,
        gl.COLOR_ATTACHMENT0 + i,
        gl.TEXTURE_2D,
        this._textures[i].texture,
        0
      );
      buffers.push(gl[`COLOR_ATTACHMENT${i}`]);
    }

    // multi render targets
    if (this.GL.multiRenderTargetSupport) {
      gl.drawBuffers(buffers);
    }
    /*

    // depth textures
    gl.framebufferTexture2D(
      target,
      gl.DEPTH_ATTACHMENT,
      gl.TEXTURE_2D,
      this.glDepthTexture.texture,
      0
    );
*/
    const FBOstatus = gl.checkFramebufferStatus(gl.FRAMEBUFFER);
    if (FBOstatus !== gl.FRAMEBUFFER_COMPLETE) {
      console.error("FBOstatus", WebGLNumber[FBOstatus], FBOstatus);
      console.error("GL_FRAMEBUFFER_COMPLETE failed, CANNOT use Framebuffer");
    }

    // UNBIND
    gl.bindTexture(gl.TEXTURE_2D, null);
    gl.bindRenderbuffer(gl.RENDERBUFFER, null);
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
  }

  /**
   * Initialize the textures
   *
   */
  _initTextures() {
    for (let i = 0; i < this._numTargets; i++) {
      this._textures.push(this._createTexture());
    }

    // depth texture
    this.glDepthTexture = this._createTexture(
      WebGLConst.DEPTH_COMPONENT16,
      WebGLConst.UNSIGNED_INT,
      WebGLConst.DEPTH_COMPONENT,
      {
        minFilter: WebGLConst.NEAREST,
        magFilter: WebGLConst.NEAREST,
        mipmap: false,
      }
    );
  }

  /**
   * Create texture
   *
   * @param {GLenum} mInternalformat GLenum value of the internal format
   * @param {GLenum} mTexelType GLenum value of texel type
   * @param {GLenum} mFormat GLenum value of the format
   * @param {object} mParameters the texture parameters
   */
  _createTexture(mInternalformat, mTexelType, mFormat, mParameters = {}) {
    const parameters = Object.assign({}, this._parameters);
    Object.assign(parameters, mParameters);

    if (!mFormat) {
      mFormat = mInternalformat;
    }

    const texture = new GLTexture(null, parameters, this._width, this._height);
    return texture;
  }

  /**
   * Destroy the framebuffer
   *
   */
  destroy() {
    const { gl } = this.GL;

    // delete all textures
    // delete depth texture
    // delete framebuffer

    this.GL.frameBufferCount--;
  }

  // getter & setters

  get texture() {
    return this._textures[0];
  }

  get depthTexture() {
    return this.glDepthTexture;
  }

  /**
   * Set the min filter of the texture
   *
   * @param {GLenum} mValue GLenum value of the min filter
   */
  set minFilter(mValue) {
    this._params.minFilter = mValue;
    this._parametersState.set(MIN_FILTER, 1);
    webgl2FilterCheck(this._params);
  }

  /**
   * Get the min filter of the texture
   *
   * @returns {GLenum} the min filter value
   */
  get minFilter() {
    return this._params.minFilter;
  }

  /**
   * Set the mag filter of the texture
   *
   * @param {GLenum} mValue GLenum value of the mag filter
   */
  set magFilter(mValue) {
    this._params.magFilter = mValue;
    this._parametersState.set(MAG_FILTER, 1);
    webgl2FilterCheck(this._params);
  }

  /**
   * Get the mag filter of the texture
   *
   * @returns {GLenum} the mag filter value
   */
  get magFilter() {
    return this._params.magFilter;
  }

  /**
   * Set the s-coordinate of the wrapping
   *
   * @param {GLenum} mValue GLenum value of the wrapping
   */
  set wrapS(mValue) {
    this._params.wrapS = mValue;
    this._parametersState.set(WRAP_S, 1);
  }

  /**
   * Get the s-coordinate of the wrapping
   *
   * @returns {GLenum} the value of s-coordinate of the wrapping
   */
  get wrapS() {
    return this._params.wrapS;
  }

  /**
   * Set the t-coordinate of the wrapping
   *
   * @param {GLenum} mValue GLenum value of the wrapping
   */
  set wrapT(mValue) {
    this._params.wrapT = mValue;
    this._parametersState.set(WRAP_T, 1);
  }

  /**
   * Get the t-coordinate of the wrapping
   *
   * @returns {GLenum} the value of t-coordinate of the wrapping
   */
  get wrapT() {
    return this._params.wrapT;
  }

  /**
   * Get the width of the texture
   *
   * @returns {number} the width of the texture
   */
  get width() {
    return this._width;
  }

  /**
   * Get the height of the texture
   *
   * @returns {number} the height of the texture
   */
  get height() {
    return this._height;
  }
}

export { FrameBuffer };
