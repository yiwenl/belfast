export { checkWebGL2 } from "./checkWebGL2";
export { isMobile } from "./isMobile";
export { getWebGLContext } from "./getWebGLContext";

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
