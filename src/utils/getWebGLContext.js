const getWebGLContext = (mCanvas, mParams, mWebGL2 = false) => {
  if (mWebGL2) {
    return (
      mCanvas.getContext("webgl2", mParams) ||
      mCanvas.getContext("experimental-webgl2", mParams)
    );
  } else {
    return (
      mCanvas.getContext("webgl", mParams) ||
      mCanvas.getContext("experimental-webgl", mParams)
    );
  }
};

export { getWebGLContext };
