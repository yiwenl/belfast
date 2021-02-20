import {
  GL,
  Scene,
  Draw,
  Geom,
  DrawAxis,
  DrawDotsPlane,
  DrawCopy,
  GLTexture,
  GLCubeTexture,
  parseDds,
  PBRShader,
} from "alfrid";
import Config from "./Config";
import Assets from "./Assets";
// import { PBRShader } from "./PBRShader";

import vs from "shaders/skybox.vert";
import fs from "shaders/skybox.frag";

class SceneApp extends Scene {
  constructor() {
    super();

    this.orbitalControl.rx.value = this.orbitalControl.ry.value = 0.3;
    this.orbitalControl.radius.value = 8;
    this.resize();
  }

  _initTextures() {
    const getCubeTexture = (mId) => {
      const oTexture = Assets.get(mId);
      const dataTexture = parseDds(oTexture);
      const { mipmapCount } = dataTexture[0];
      const numLevel = dataTexture.length / 6;
      let sourceTexture = [];

      if (mipmapCount === 99) {
        // sourceTexture = dataTexture.map((o) => o.data);
        for (let i = 0; i < 6; i++) {
          sourceTexture.push(dataTexture[i * numLevel]);
        }
      } else {
        sourceTexture = dataTexture.map((o) => o.data);
      }

      return new GLCubeTexture(
        sourceTexture,
        { type: GL.FLOAT },
        dataTexture[0].shape[0],
        dataTexture[0].shape[1]
      );
    };

    this._textureLUT = new GLTexture(Assets.get("brdfLUT"));
    const { env } = Config;

    this._textureRad = getCubeTexture(`${env}_radiance`);
    this._textureIrr = getCubeTexture(`${env}_irradiance`);
  }

  _initViews() {
    this._dAxis = new DrawAxis();
    this._dDots = new DrawDotsPlane();
    this._dCopy = new DrawCopy();

    let s = 20;
    this._drawSkybox = new Draw()
      .setMesh(Geom.cube(s, s, s, true))
      .useProgram(vs, fs)
      .bindTexture("texture", this._texture, 0);

    s = 2.0;
    const shader = new PBRShader();
    console.log(shader);
    this._draw = new Draw().setMesh(Geom.sphere(s, 36)).useProgram(shader);
    shader.lutMap = this._textureLUT;
    shader.radianceMap = this._textureRad;
    shader.irradianceMap = this._textureIrr;
    shader.baseColor = [1, 0, 0];
    shader.diffuseOffset = 0.0;

    this.shaderPBR = shader;
  }

  render() {
    this.shaderPBR.baseColor = Config.color.map((v) => v / 255);
    this.shaderPBR.roughness = Config.roughness;
    this.shaderPBR.metallic = Config.metallic;
    this.shaderPBR.exposure = Config.exposure;
    this.shaderPBR.cameraPosition = this.camera.position;
    GL.clear(0, 0, 0, 1);
    // this._drawSkybox.bindTexture("texture", this._textureRad, 0).draw();
    this._drawSkybox.bindTexture("texture", this._textureIrr, 0).draw();

    this._dAxis.draw();
    this._dDots.draw();

    // temp, bind all texture
    this.shaderPBR.bindAllTextures();
    this._draw.draw();
  }
}

export default SceneApp;
