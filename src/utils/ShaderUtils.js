import { glMatrix, vec3, mat4 } from "gl-matrix";

export const addLineNumbers = (string) => {
  const lines = string.split("\n");
  for (let i = 0; i < lines.length; i++) {
    lines[i] = `${i + 1}: ${lines[i]}`;
  }
  return lines.join("\n");
};

export const uniformMapping = {
  float: "uniform1f",
  vec2: "uniform2fv",
  vec3: "uniform3fv",
  vec4: "uniform4fv",
  int: "uniform1i",
  ivec2: "uniform2i",
  ivec3: "uniform3i",
  ivec4: "uniform4i",
  mat2: "uniformMatrix2fv",
  mat3: "uniformMatrix3fv",
  mat4: "uniformMatrix4fv",
};

export const cloneValue = (mValue) => {
  if (typeof mValue === "number") {
    return mValue;
  }
  if (mValue.slice) {
    return mValue.slice(0);
  } else {
    return new Float32Array(mValue);
  }
};

export const equals = (a, b) => {
  if (typeof a === "number") {
    return glMatrix.equals(a, b);
  }

  if (a.length !== b.length) {
    console.warn("Arrays have different length :", a, b);
    return false;
  }

  let _isEqual = true;
  a.forEach((v, i) => {
    _isEqual = glMatrix.equals(v, b[i]) && _isEqual;
  });
  return _isEqual;
};

export const getUniformType = (mValue) => {
  const isArray = typeof mValue === "object";

  const getArrayUniformType = function(mValue) {
    if (mValue.length === 9) {
      return "mat3";
    } else if (mValue.length === 16) {
      return "mat4";
    } else {
      return `vec${mValue.length}`;
    }
  };

  if (!isArray) {
    return "float";
  } else {
    return getArrayUniformType(mValue);
  }
};
