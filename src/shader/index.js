import fsSimpleColor from "./simpleColor.frag";
import fsCopy from "./copy.frag";
import vsTriangle from "./bigTriangle.vert";

const ShaderLibs = {
  simpleColorFrag: fsSimpleColor,
  copyFrag: fsCopy,
  bigTriangleVert: vsTriangle,
};

export { ShaderLibs };
