class FrameBuffer {
  constructor(mWidth, mHeight, mParameters = {}, mNumTargets = 1) {
    this._width = mWidth;
    this._height = mHeight;
    this._parameters = mParameters;
    this._numTargets = mNumTargets;
  }

  /**
   * Bind the frame buffer
   *
   * @param {GL} mGL the GLTool instance
   */
  bind(mGL) {
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

    console.log(
      "bind Frame buffer:",
      this.GL.multiRenderTargetSupport,
      this.GL.maxMultiRenderTargets
    );
  }

  /**
   * Unbind the frame buffer
   *
   */
  unbind() {
    return;
  }

  /**
   * Initialize the textures
   *
   */
  _initTextures() {
    return;
  }

  // getter & setters

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
