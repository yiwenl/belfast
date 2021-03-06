// basic.vert

precision highp float;
attribute vec3 aVertexPosition;
attribute vec3 aExtra;
attribute vec2 aTextureCoord;

varying vec3 vPosition;
varying vec3 vExtra;

void main(void) {
    gl_Position = vec4(aTextureCoord, 0.0, 1.0);
    vPosition = aVertexPosition;
    vExtra = aExtra;
    gl_PointSize = 1.0;
}