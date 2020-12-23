import { glMatrix } from "gl-matrix";
export { checkWebGL2 } from "./checkWebGL2";
export { getExtensions } from "./getExtensions";
export { isMobile } from "./isMobile";
// export { getWebGLContext } from "./getWebGLContext";

export const checkViewport = (viewport, x, y, w, h) => {
  let hasChanged = false;
  if (x !== viewport[0]) {
    hasChanged = true;
  }
  if (y !== viewport[1]) {
    hasChanged = true;
  }
  if (w !== viewport[2]) {
    hasChanged = true;
  }
  if (h !== viewport[3]) {
    hasChanged = true;
  }
  return hasChanged;
};

export const equals = (a, b) => {
  if (typeof a === "number") {
    return glMatrix.equals(a, b);
  }

  if (a.length !== b.length) {
    return false;
  }

  let _isEqual = true;
  a.forEach((v, i) => {
    _isEqual = glMatrix.equals(v, b[i]) && _isEqual;
  });
  return _isEqual;
};
