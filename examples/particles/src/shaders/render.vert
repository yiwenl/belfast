#version 300 es

precision highp float;
in vec3 aVertexPosition;

uniform mat4 uModelMatrix;
uniform mat4 uViewMatrix;
uniform mat4 uProjectionMatrix;
uniform mat4 uShadowMatrix;

uniform vec2 uViewport;
uniform sampler2D texturePos;

out vec4 vShadowCoord;

const float radius = 0.02;

void main(void) {
    vec3 pos = texture(texturePos, aVertexPosition.xy).xyz;
    vec4 wsPos = uModelMatrix * vec4(pos, 1.0);
    gl_Position = uProjectionMatrix * uViewMatrix * wsPos;

    float distOffset = uViewport.y * uProjectionMatrix[1][1] * radius / gl_Position.w;
    gl_PointSize = distOffset;

    vShadowCoord = uShadowMatrix * wsPos;
}