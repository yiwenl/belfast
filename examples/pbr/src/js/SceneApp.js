import {
  GL,
  Scene,
  Draw,
  Geom,
  DrawAxis,
  DrawDotsPlane,
  DrawCopy,
  PBRShader,
} from "alfrid";
import Config from "./Config";
import Assets from "./Assets";
import { mat4 } from "gl-matrix";
// import { PBRShader } from "./PBRShader";

import vs from "shaders/skybox.vert";
import fs from "shaders/skybox.frag";

class SceneApp extends Scene {
  constructor() {
    super();

    this.orbitalControl.rx.value = this.orbitalControl.ry.value = 0.3;
    this.orbitalControl.radius.value = 8;

    this.camera.setPerspective(Math.PI / 3, GL.aspectRatio, 0.1, 100);
    this.resize();
  }

  _initTextures() {
    this._textureLUT = Assets.get("brdfLUT");
    const { env } = Config;

    this._textureRad = Assets.get(`${env}_radiance`);
    this._textureIrr = Assets.get(`${env}_irradiance`);
  }

  _initViews() {
    this._dAxis = new DrawAxis();
    this._dDots = new DrawDotsPlane();
    this._dCopy = new DrawCopy();

    let s = 50;
    this._drawSkybox = new Draw()
      .setMesh(Geom.cube(s, s, s, true))
      .useProgram(vs, fs)
      .bindTexture("texture", this._texture, 0);

    s = 0.4;
    const shader = new PBRShader();
    this._draw = new Draw().setMesh(Geom.sphere(s, 36)).useProgram(shader);
    shader.lutMap = this._textureLUT;
    shader.radianceMap = this._textureRad;
    shader.irradianceMap = this._textureIrr;
    shader.baseColor = [1, 0, 0];
    shader.diffuseOffset = 0;

    this.shaderPBR = shader;
  }

  render() {
    GL.clear(0, 0, 0, 1);
    this._drawSkybox.bindTexture("texture", this._textureIrr, 0).draw();
    this._dAxis.draw();
    this._dDots.draw();

    this.shaderPBR.bindAllTextures();
    this.shaderPBR.baseColor = Config.color.map((v) => v / 255);
    this.shaderPBR.exposure = Config.exposure;
    this.shaderPBR.cameraPosition = this.camera.position;

    const mtx = mat4.create();
    const num = 10;

    for (let i = 0; i <= num; i++) {
      for (let j = 0; j <= num; j++) {
        mat4.identity(mtx);
        mat4.translate(mtx, mtx, [-num / 2 + i, 0, -num / 2 + j]);
        GL.setModelMatrix(mtx);

        this.shaderPBR.roughness = i / num;
        this.shaderPBR.metallic = j / num;
        this._draw.draw();
      }
    }
  }
}

export default SceneApp;
