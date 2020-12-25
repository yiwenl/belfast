precision highp float;

varying vec2 vUV;
uniform sampler2D texture;

void main(void) {
    gl_FragColor = vec4(vUV, 0.0, 1.0);
    gl_FragColor = texture2D(texture, vUV);
}