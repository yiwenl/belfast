#version 300 es

precision highp float;
in vec4 vShadowCoord;
out vec4 oColor;
uniform sampler2D textureDepth;

#define uMapSize vec2(2048.0)


float samplePCF3x3( vec4 sc )
{
    const int s = 2;
    float shadow = 0.0;
    float bias = 0.005;
    float threshold = sc.z - bias;

    shadow += step(threshold, textureProjOffset( textureDepth, sc, ivec2(-s,-s) ).r);
    shadow += step(threshold, textureProjOffset( textureDepth, sc, ivec2(-s, 0) ).r);
    shadow += step(threshold, textureProjOffset( textureDepth, sc, ivec2(-s, s) ).r);
    shadow += step(threshold, textureProjOffset( textureDepth, sc, ivec2( 0,-s) ).r);
    shadow += step(threshold, textureProjOffset( textureDepth, sc, ivec2( 0, 0) ).r);
    shadow += step(threshold, textureProjOffset( textureDepth, sc, ivec2( 0, s) ).r);
    shadow += step(threshold, textureProjOffset( textureDepth, sc, ivec2( s,-s) ).r);
    shadow += step(threshold, textureProjOffset( textureDepth, sc, ivec2( s, 0) ).r);
    shadow += step(threshold, textureProjOffset( textureDepth, sc, ivec2( s, s) ).r);
    return shadow/9.0;;
}



float rand(vec4 seed4) {
	float dot_product = dot(seed4, vec4(12.9898,78.233,45.164,94.673));
	return fract(sin(dot_product) * 43758.5453);
}


void main(void) {
    if(distance(gl_PointCoord, vec2(.5)) > .5) discard;


    vec4 shadowCoord    = vShadowCoord / vShadowCoord.w;
	float s             = samplePCF3x3(shadowCoord);
    s = mix(s, 1.0, .25);

    oColor = vec4(vec3(s), 1.0);
    // oColor = vec4(shadowCoord.xyz, 1.0);
}