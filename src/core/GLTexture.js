import { GL } from "./GL";
import WebglNumber from "../utils/WebglNumber";
import WebglConst from "../utils/WebglConst";
import { isPowerOfTwo, getTextureParameters } from "../utils/TextureUtils";

class GLTexture {
  constructor(mSource, mParam = {}, mWidth = 0, mHeight = 0) {
    this._source = mSource;
    this._getDimension(mSource, mWidth, mHeight);
    this._texelType = WebglConst.UNSIGNED_BYTE;
    this._params = getTextureParameters(mParam, this._width, this._height);
    this._checkMipmap();
    // this.showParameters();

    // states
    this._hasParameterChanged = false;
  }

  bind(mIndex, mGL) {
    if (mGL !== undefined && this.GL !== undefined && mGL !== this.GL) {
      console.error(
        "this shader has been bind to a different WebGL Rendering Context",
        this.GL.id
      );
      return;
    }

    console.log("bind texture :", mGL.id);

    this.GL = mGL || GL;
    const { gl } = this.GL;
    if (!this._texture) {
      this._uploadTexture();
    }

    gl.activeTexture(gl.TEXTURE0 + mIndex);
    gl.bindTexture(gl.TEXTURE_2D, this._texture);
  }

  updateTexture(mSource) {
    this._source = mSource;
    this._uploadTexture();
  }

  generateMipmap() {
    return;
  }

  _uploadTexture() {
    const { gl } = this.GL;

    if (!this._texture) {
      this._texture = gl.createTexture();
    }
    gl.bindTexture(gl.TEXTURE_2D, this._texture);
    gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, true);

    gl.texImage2D(
      gl.TEXTURE_2D,
      0,
      this._params.internalFormat,
      this._params.format,
      this._texelType,
      this._source
    );

    // texture parameters
    gl.texParameteri(
      gl.TEXTURE_2D,
      gl.TEXTURE_MAG_FILTER,
      this._params.magFilter
    );
    gl.texParameteri(
      gl.TEXTURE_2D,
      gl.TEXTURE_MIN_FILTER,
      this._params.minFilter
    );
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, this._params.wrapS);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, this._params.wrapT);
    gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, this._premultiplyAlpha);

    if (this._generateMipmap) {
      gl.generateMipmap(gl.TEXTURE_2D);
    }
  }

  _getDimension(mSource, mWidth, mHeight) {
    if (mSource) {
      // for html image / video element
      this._width = mSource.width || mSource.videoWidth;
      this._height = mSource.height || mSource.videoWidth;

      // for manual width / height settings
      this._width = this._width || mWidth;
      this._height = this._height || mHeight;

      // auto detect ( data array) ? not sure is good idea ?
      // todo : check HDR
      if (!this._width || !this._height) {
        this._width = this._height = Math.sqrt(mSource.length / 4);
        // console.log('Auto detect, data dimension : ', this._width, this._height);
      }
    } else {
      this._width = mWidth;
      this._height = mHeight;
    }
  }

  _checkMipmap() {
    this._generateMipmap = this._params.mipmap;

    if (!(isPowerOfTwo(this._width) && isPowerOfTwo(this._height))) {
      this._generateMipmap = false;
    }

    const minFilter = WebglNumber[this._params.minFilter];
    if (minFilter.indexOf("MIPMAP") === -1) {
      this._generateMipmap = false;
    }
  }

  showParameters() {
    /*
    console.log(
      "Source type : ",
      WebglNumber[this._sourceType] || this._sourceType
    );
    console.log("Texel type:", WebglNumber[this.texelType]);
    */
    console.log("Dimension :", this._width, this._height);
    for (const s in this._params) {
      console.log(s, WebglNumber[this._params[s]] || this._params[s]);
    }
  }

  // getter & setters
  get width() {
    return this._width;
  }

  get height() {
    return this._height;
  }
}

export { GLTexture };
