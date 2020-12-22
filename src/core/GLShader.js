import { GL } from "../core/GL";
import { addLineNumbers } from "../utils/ShaderUtils";
import vsDefault from "../shader/basic.vert";
import fsDefault from "../shader/basic.frag";

function GLShader(mVertexShader, mFragmentShader) {
  this.vertexShader = mVertexShader || vsDefault;
  this.fragmentShader = mFragmentShader || fsDefault;
  this.GL;
  this.shaderProgram;

  /**
   * Bind the current shader
   *
   * @param {GL} mGL the GLTool instance
   */
  this.bind = function(mGL) {
    if (this.GL !== undefined && mGL !== this.GL) {
      console.error(
        "this shader has been bind to a different WebGL Rendering Context"
      );
      return;
    }

    this.GL = mGL || GL;
    if (!this.shaderProgram) {
      const vsShader = createShaderProgram(this.vertexShader, true);
      const fsShader = createShaderProgram(this.fragmentShader, false);
      attachShaderProgram(vsShader, fsShader);
    }

    this.GL.useShader(this);
  };

  /**
   * Set the uniform of the shader
   *
   * @param {string|object} mName the name of the uniform
   * @param {string} mType the type of the uniform
   * @param {number|[numbers]} mValue the value of the uniform
   */
  this.uniform = function(mName, mType, mValue) {};

  /**
   * Destroy the current shader
   *
   */
  this.destroy = function() {
    const { gl } = this.GL;
    gl.deleteProgram(this.shaderProgram);
    this.GL.shaderCount--;
  };

  /**
   * Create & Compile shader
   *
   * @param {string} mShaderStr the shader program text
   * @param {boolean} isVertexShader is vertex shader or not
   */
  const createShaderProgram = (mShaderStr, isVertexShader) => {
    const { GL } = this;
    const { gl } = this.GL;
    const shaderType = isVertexShader ? GL.VERTEX_SHADER : GL.FRAGMENT_SHADER;
    const shader = gl.createShader(shaderType);

    gl.shaderSource(shader, mShaderStr);
    gl.compileShader(shader);

    if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
      console.warn("Error in Shader : ", gl.getShaderInfoLog(shader));
      console.log(addLineNumbers(mShaderStr));
      return null;
    }

    return shader;
  };

  /**
   * Attach shader
   *
   * @param {glShader} mVertexShader the vertex shader
   * @param {glShader} mFragmentShader the fragment shader
   */
  const attachShaderProgram = (mVertexShader, mFragmentShader) => {
    const { gl } = this.GL;

    this.shaderProgram = gl.createProgram();
    gl.attachShader(this.shaderProgram, mVertexShader);
    gl.attachShader(this.shaderProgram, mFragmentShader);
    gl.deleteShader(mVertexShader);
    gl.deleteShader(mFragmentShader);

    gl.linkProgram(this.shaderProgram);
    this.GL.shaderCount++;
  };
}

export { GLShader };
