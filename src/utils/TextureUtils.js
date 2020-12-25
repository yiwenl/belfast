import WebglConst from "./WebglConst";

export const isPowerOfTwo = (x) => {
  return x !== 0 && !(x & (x - 1));
};

export const getTextureParameters = function(mParams, mWidth, mHeight) {
  if (!mParams.minFilter) {
    let minFilter = WebglConst.LINEAR;
    if (mWidth && mWidth) {
      if (isPowerOfTwo(mWidth) && isPowerOfTwo(mHeight)) {
        minFilter = WebglConst.NEAREST_MIPMAP_LINEAR;
      }
    }

    mParams.minFilter = minFilter;
  }

  mParams.mipmap = mParams.mipmap === undefined ? true : mParams.mipmap;
  mParams.magFilter = mParams.magFilter || WebglConst.LINEAR;
  mParams.wrapS = mParams.wrapS || WebglConst.CLAMP_TO_EDGE;
  mParams.wrapT = mParams.wrapT || WebglConst.CLAMP_TO_EDGE;
  mParams.internalFormat = mParams.internalFormat || WebglConst.RGBA;
  mParams.format = mParams.format || WebglConst.RGBA;
  mParams.premultiplyAlpha =
    mParams.premultiplyAlpha === undefined ? false : mParams.premultiplyAlpha;
  mParams.level = mParams.level || 0;

  // if (WebglConst.webgl2 && mParams.type === WebglConst.FLOAT) {
  //   mParams.internalFormat = WebglConst.WebglConst.RGBA32F;
  //   mParams.mipmap = false;
  // }
  return mParams;
};
