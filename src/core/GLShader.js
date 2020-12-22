import { GL } from "../core/GL";
import { addLineNumbers } from "../utils/ShaderUtils";
import vsDefault from "../shader/basic.vert";
import fsDefault from "../shader/basic.frag";

function GLShader(mVertexShader, mFragmentShader) {
  /**
   * Public Properties
   *
   */
  this.vertexShader = mVertexShader || vsDefault;
  this.fragmentShader = mFragmentShader || fsDefault;
  this.GL;
  this.shaderProgram;

  /**
   * Private Properties
   *
   */

  /**
   * Bind the current shader
   *
   * @param {GL} mGL the GLTool instance
   */
  this.bind = function(mGL) {
    this.GL = mGL || GL;
    console.log("Bind shader", this.GL.id);
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
   */
  this.uniform = function() {};

  /**
   * Destroy the current shader
   *
   */
  this.destroy = function() {};

  /**
   * Private Methods
   *
   */

  /**
   * Bind the current shader
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
