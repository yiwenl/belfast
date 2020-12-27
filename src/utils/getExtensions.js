import getAndApplyExtension from "./getAndApplyExtension";

const extensionsWebGL1 = [
  "EXT_shader_texture_lod",
  "EXT_sRGB",
  "EXT_frag_depth",
  "OES_texture_float",
  "OES_texture_half_float",
  "OES_texture_float_linear",
  "OES_texture_half_float_linear",
  "OES_standard_derivatives",
  "WEBGL_depth_texture",
  "EXT_texture_filter_anisotropic",
  "OES_vertex_array_object",
  "ANGLE_instanced_arrays",
  "WEBGL_draw_buffers",
];

const extensionsWebGL2 = [
  "EXT_color_buffer_float",
  "EXT_texture_filter_anisotropic",
];

/**
 * Clear WebGL Context
 *
 * @param {object} mGL the GLTool Instance
 * @returns {object} the object contains all extensions
 */

const getExtensions = (mGL) => {
  const { gl } = mGL;
  const isWebGL2 = gl instanceof WebGL2RenderingContext;
  const extensions = {};
  const extensionsList = isWebGL2 ? extensionsWebGL2 : extensionsWebGL1;
  extensionsList.forEach((ext) => {
    extensions[ext] = gl.getExtension(ext);
  });

  if (!isWebGL2) {
    // only IE not support
    // caniuse.com/?search=OES_vertex_array_object
    if (!extensions["OES_vertex_array_object"]) {
      console.error("OES_vertex_array_object extension is not supported");
    }
    getAndApplyExtension(gl, "OES_vertex_array_object");
    getAndApplyExtension(gl, "ANGLE_instanced_arrays");
    getAndApplyExtension(gl, "WEBGL_draw_buffers");
  }

  // ANISOTROPY Filter Check
  const extAnisotropic = extensions["EXT_texture_filter_anisotropic"];
  if (extAnisotropic) {
    mGL.maxAnisotropy = gl.getParameter(
      extAnisotropic.MAX_TEXTURE_MAX_ANISOTROPY_EXT
    );
  }

  // Draw Buffers
  mGL.multiRenderTargetSupport = !!mGL.gl.drawBuffers;

  if (mGL.multiRenderTargetSupport) {
    const MAX_DRAW_BUFFERS =
      mGL.gl.MAX_DRAW_BUFFERS ||
      extensions["WEBGL_draw_buffers"].MAX_DRAW_BUFFERS_WEBGL;
    mGL.maxMultiRenderTargets = gl.getParameter(MAX_DRAW_BUFFERS);
  }

  return extensions;
};

export { getExtensions };
