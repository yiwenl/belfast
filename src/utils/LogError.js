const Errors = {
  DRAW_BUFFERS: `This browser doesn't support multi render targets : WEBGL_draw_buffers`,
  FRAMEBUFFER_CONTEXT: `This browser doesn't support multi render targets : WEBGL_draw_buffers`,
  SHADER_CONTEXT: `This browser doesn't support multi render targets : WEBGL_draw_buffers`,
  TEXTURE_CONTEXT: `This browser doesn't support multi render targets : WEBGL_draw_buffers`,
};

const logError = (mMessage, mExtra = "") => {
  console.error(mMessage, mExtra);
};

export default logError;
export { Errors };
