import fsSimpleColor from "./simpleColor.frag";
import fsCopy from "./copy.frag";
import vsGeneral from "./general.vert";
import vsTriangle from "./bigTriangle.vert";

const ShaderLibs = {
  simpleColorFrag: fsSimpleColor,
  copyFrag: fsCopy,
  bigTriangleVert: vsTriangle,
  generalVert: vsGeneral,
};

export { ShaderLibs };
