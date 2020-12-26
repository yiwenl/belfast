import { WebGLConst } from "./WebGLConst";
import { WebGLNumber } from "./WebGLNumber";

export const isPowerOfTwo = (x) => {
  return x !== 0 && !(x & (x - 1));
};

export const getTextureParameters = function(mParams, mWidth, mHeight) {
  if (!mParams.minFilter) {
    let minFilter = WebGLConst.LINEAR;
    if (mWidth && mWidth) {
      if (isPowerOfTwo(mWidth) && isPowerOfTwo(mHeight)) {
        minFilter = WebGLConst.NEAREST_MIPMAP_LINEAR;
      }
    }

    mParams.minFilter = minFilter;
  }

  mParams.mipmap = mParams.mipmap === undefined ? true : mParams.mipmap;
  mParams.magFilter = mParams.magFilter || WebGLConst.LINEAR;
  mParams.wrapS = mParams.wrapS || WebGLConst.CLAMP_TO_EDGE;
  mParams.wrapT = mParams.wrapT || WebGLConst.CLAMP_TO_EDGE;
  mParams.internalFormat = mParams.internalFormat || WebGLConst.RGBA;
  mParams.format = mParams.format || WebGLConst.RGBA;
  mParams.premultiplyAlpha =
    mParams.premultiplyAlpha === undefined ? false : mParams.premultiplyAlpha;
  mParams.level = mParams.level || 0;

  mParams.type = mParams.type || WebGLConst.UNSIGNED_BYTE;
  return mParams;
};

export const isSourceHtmlElement = (mSource) => {
  return (
    mSource instanceof HTMLImageElement ||
    mSource instanceof HTMLCanvasElement ||
    mSource instanceof HTMLVideoElement
  );
};

export const checkSource = (mSource, mParams) => {
  let flag = true;

  // source check
  if (mSource.constructor.name === "Array") {
    console.error(
      "Please convert texture source to Unit8Array or Float32Array"
    );
    flag = false;
  }

  // type check
  if (mParams.type === undefined) {
    if (mSource.constructor.name !== "Uint8Array") {
      console.error(
        "Using none Unit8Array, pleaes specify type in the texture parameters"
      );
    }
  }

  return flag;
};

export const webgl2TextureCheck = (mGL, mParams) => {
  if (!mGL.webgl2) {
    return;
  }

  if (mParams.type !== WebGLConst.UNSIGNED_BYTE) {
    // floating point texture
    if (mParams.type === WebGLConst.HALF_FLOAT) {
      /**
       * enum OES_HALF_FLOAT  !== webgl2.HALF_FLOAT
       *
       */
      mParams.type = mGL.gl.HALF_FLOAT;
      mParams.internalFormat = WebGLConst.RGBA16F;
    } else {
      mParams.internalFormat = WebGLConst.RGBA32F;
    }
  }
};
