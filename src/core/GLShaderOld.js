import { GL } from "../core/GL";
import { addLineNumbers } from "../utils/ShaderUtils";
import vsDefault from "../shader/basic.vert";
import fsDefault from "../shader/basic.frag";

class GLShader {
  constructor(mVertexShader, mFragmentShader) {
    this.vertexShader = mVertexShader || vsDefault;
    this.fragmentShader = mFragmentShader || fsDefault;
  }

  // bind Shader to GL
  bind(mGL) {
    this.GL = mGL || GL;

    if (!this.shaderProgram) {
      const vsShader = this._createShaderProgram(this.vertexShader, true);
      const fsShader = this._createShaderProgram(this.fragmentShader, false);
      this._attachShaderProgram(vsShader, fsShader);
    }

    this.GL.useShader(this);
    this.GL.gl.useProgram(this.shaderProgram);
  }

  // create shader program
  _createShaderProgram(mShaderStr, isVertexShader) {
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
  }

  // attach shader program
  _attachShaderProgram(mVertexShader, mFragmentShader) {
    const { gl } = this.GL;

    this.shaderProgram = gl.createProgram();
    gl.attachShader(this.shaderProgram, mVertexShader);
    gl.attachShader(this.shaderProgram, mFragmentShader);
    gl.deleteShader(mVertexShader);
    gl.deleteShader(mFragmentShader);

    gl.linkProgram(this.shaderProgram);
    this.GL.shaderCount++;
  }

  // uniform
  uniform() {
    // console.log("Uniform", arguments.length);
  }

  // destroy shader
  destroy() {
    const { gl } = this.GL;
    gl.deleteProgram(this.shaderProgram);
    this.GL.shaderCount--;
  }
}

export { GLShader };
