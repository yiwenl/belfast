// basic.vert

#define SHADER_NAME BASIC_VERTEX

precision highp float;
attribute vec3 aVertexPosition;
attribute vec2 aTextureCoord;
// attribute vec3 aNormal;

// uniform mat4 uModelMatrix;
// uniform mat4 uViewMatrix;
// uniform mat4 uProjectionMatrix;

// varying vec2 vTextureCoord;
// varying vec3 vNormal;

void main(void) {
  gl_Position = vec4(aVertexPosition, 1.0);
  // vTextureCoord = aTextureCoord;
  // vNormal = aNormal;

  // gl_PointSize = 5.0;
}