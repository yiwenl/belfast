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

export const cloneArray = (mArray) => {
  if (mArray.slice) {
    return mArray.slice(0);
  } else {
    return new Float32Array(mArray);
  }
};

export const isSame = (array1, array2) => {
  if (array1.length !== array2.length) {
    return false;
  }

  for (let i = 0; i < array1.length; i++) {
    if (array1[i] !== array2[i]) {
      return false;
    }
  }

  return true;
};

export const getUniformType = (mValue) => {
  const isArray = !!mValue.concat;

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
