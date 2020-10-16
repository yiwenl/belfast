export const getBuffer = function(attr, gl) {
  let buffer;
  console.log(gl);

  if (attr.buffer !== undefined) {
    buffer = attr.buffer;
  } else {
    buffer = gl.createBuffer();
    attr.buffer = buffer;
  }

  return buffer;
};

export const formBuffer = function(mData, mNum) {
  const ary = [];

  for (let i = 0; i < mData.length; i += mNum) {
    const o = [];
    for (let j = 0; j < mNum; j++) {
      o.push(mData[i + j]);
    }

    ary.push(o);
  }

  return ary;
};

export const getAttribLoc = (gl, shaderProgram, name) => {
  if (shaderProgram.cacheAttribLoc === undefined) {
    shaderProgram.cacheAttribLoc = {};
  }
  if (shaderProgram.cacheAttribLoc[name] === undefined) {
    shaderProgram.cacheAttribLoc[name] = gl.getAttribLocation(
      shaderProgram,
      name
    );
  }

  return shaderProgram.cacheAttribLoc[name];
};