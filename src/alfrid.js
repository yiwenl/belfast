// core
export { GL, GLTool } from "./core/GL";
export { GLShader } from "./core/GLShader";
export { Mesh } from "./core/Mesh";
// export { Mesh } from "./core/MeshOld";

// utils
export { checkWebGL2 } from "./utils/checkWebGL2";
export { getWebGLContext } from "./utils/getWebGLContext";

// polyfill fixes
import "./utils/polyfixes";
