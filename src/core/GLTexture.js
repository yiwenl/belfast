import { GL } from "./GL";
import {
  isPowerOfTwo,
  getTextureParameters,
  isSourceHtmlElement,
  checkSource,
  webgl2TextureCheck,
  webgl2FilterCheck,
} from "../utils/TextureUtils";
import { WebGLNumber } from "../utils/WebGLNumber";
import { BitSwitch } from "../utils/BitSwitch";

const MIN_FILTER = 0;
const MAG_FILTER = 1;
const WRAP_S = 2;
const WRAP_T = 3;

class GLTexture {
  constructor(mSource, mParam = {}, mWidth = 0, mHeight = 0) {
    this._source = mSource;
    this._isHtmlElement = isSourceHtmlElement(this._source);
    if (!this._isHtmlElement) {
      if (!checkSource(mSource, mParam)) {
        return;
      }
    }

    this._getDimension(mSource, mWidth, mHeight);
    this._params = getTextureParameters(mParam, this._width, this._height);
    this._checkMipmap();
    // this.showParameters();

    // states
    this._parametersState = new BitSwitch(0);
  }

  bind(mIndex, mGL) {
    if (mGL !== undefined && this.GL !== undefined && mGL !== this.GL) {
      console.error(
        "this shader has been bind to a different WebGL Rendering Context",
        this.GL.id
      );
      return;
    }

    this.GL = mGL || GL;
    const { gl } = this.GL;

    if (!this._texture) {
      webgl2TextureCheck(this.GL, this._params);
      this._uploadTexture();
    }

    gl.activeTexture(gl.TEXTURE0 + mIndex);
    gl.bindTexture(gl.TEXTURE_2D, this._texture);

    this._checkParameters();
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
      this.GL.textureCount++;
    }
    gl.bindTexture(gl.TEXTURE_2D, this._texture);
    gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, true);

    if (this._isHtmlElement && !this.GL.webgl2) {
      gl.texImage2D(
        gl.TEXTURE_2D,
        0,
        this._params.internalFormat,
        this._params.format,
        this._params.type,
        this._source
      );
    } else {
      gl.texImage2D(
        gl.TEXTURE_2D,
        0,
        this._params.internalFormat,
        this._width,
        this._height,
        0,
        this._params.format,
        this._params.type,
        this._source
      );
    }

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

  _checkParameters() {
    const { gl } = this.GL;
    if (this._parametersState.value > 0) {
      if (this._parametersState.get(MIN_FILTER)) {
        gl.texParameteri(
          gl.TEXTURE_2D,
          gl.TEXTURE_MIN_FILTER,
          this._params.minFilter
        );
      } else if (this._parametersState.get(MAG_FILTER)) {
        gl.texParameteri(
          gl.TEXTURE_2D,
          gl.TEXTURE_MAG_FILTER,
          this._params.magFilter
        );
      } else if (this._parametersState.get(WRAP_S)) {
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, this._params.wrapS);
      } else {
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, this._params.wrapT);
      }
    }
    this._parametersState.reset(0);
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

    const minFilter = WebGLNumber[this._params.minFilter];
    if (minFilter.indexOf("MIPMAP") === -1) {
      this._generateMipmap = false;
    }
  }

  destroy() {
    const { gl } = this.GL;
    gl.deleteTexture(this._texture);
    this.GL.textureCount--;
  }

  showParameters() {
    console.log("Dimension :", this._width, this._height);
    for (const s in this._params) {
      console.log(s, WebGLNumber[this._params[s]] || this._params[s]);
    }
  }

  // getter & setters

  set minFilter(mValue) {
    this._params.minFilter = mValue;
    this._parametersState.set(MIN_FILTER, 1);
    webgl2FilterCheck(this._params);
  }

  get minFilter() {
    return this._params.minFilter;
  }

  set magFilter(mValue) {
    this._params.magFilter = mValue;
    this._parametersState.set(MAG_FILTER, 1);
    webgl2FilterCheck(this._params);
  }

  get magFilter() {
    return this._params.magFilter;
  }

  set wrapS(mValue) {
    this._params.wrapS = mValue;
    this._parametersState.set(WRAP_S, 1);
  }

  get wrapS() {
    return this._params.wrapS;
  }

  set wrapT(mValue) {
    this._params.wrapT = mValue;
    this._parametersState.set(WRAP_T, 1);
  }

  get wrapT() {
    return this._params.wrapT;
  }

  get width() {
    return this._width;
  }

  get height() {
    return this._height;
  }
}

const getCanvas = (mColor) => {
  const canvas = document.createElement("canvas");
  canvas.width = canvas.height = 2;
  const ctx = canvas.getContext("2d");
  ctx.fillStyle = mColor;
  ctx.fillRect(0, 0, 2, 2);
  return canvas;
};

GLTexture.whiteTexture = function whiteTexture() {
  return new GLTexture(getCanvas("#fff"));
};

GLTexture.greyTexture = function greyTexture() {
  return new GLTexture(getCanvas("rgb(127, 127, 127)"));
};

GLTexture.blackTexture = function blackTexture() {
  return new GLTexture(getCanvas("#000"));
};

export { GLTexture };
