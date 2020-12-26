import { WebGLConst } from "./WebGLConst";

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

  // if (WebGLConst.webgl2 && mParams.type === WebGLConst.FLOAT) {
  //   mParams.internalFormat = WebGLConst.RGBA32F;
  //   mParams.mipmap = false;
  // }
  return mParams;
};

export const isSourceHtmlElement = (mSource) => {
  return (
    mSource instanceof HTMLImageElement ||
    mSource instanceof HTMLCanvasElement ||
    mSource instanceof HTMLVideoElement
  );
};
