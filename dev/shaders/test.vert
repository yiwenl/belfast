// basic.vert

precision highp float;
attribute vec3 aVertexPosition;
attribute vec3 aColor;

uniform mat4 uModelMatrix;
uniform mat4 uViewMatrix;
uniform mat4 uProjectionMatrix;
uniform vec3 uColors[3];

varying vec3 vColor;

void main(void) {
    gl_Position = uProjectionMatrix * uViewMatrix * uModelMatrix * vec4(aVertexPosition, 1.0);


    vec3 color = vec3(0.0);

    if(aColor.x < 0.5) {
        color = uColors[0];
    } else if(aColor.x < 1.5) {
        color = uColors[1];
    } else {
        color = uColors[2];
    }

    vColor = color;
}