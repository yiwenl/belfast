import {
  GL,
  Scene,
  Draw,
  Geom,
  DrawAxis,
  DrawDotsPlane,
  DrawCopy,
  GLCubeTexture,
} from "alfrid";
import Assets from "./Assets";

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
    const sources = ["px", "nx", "py", "ny", "pz", "nz"].map((name) =>
      Assets.get(name)
    );

    this._texture = new GLCubeTexture(sources);
  }

  _initViews() {
    console.log("init views");

    this._dAxis = new DrawAxis();
    this._dDots = new DrawDotsPlane();
    this._dCopy = new DrawCopy();

    const s = 20;
    this._drawSkybox = new Draw()
      .setMesh(Geom.cube(s, s, s, true))
      .useProgram(vs, fs)
      .bindTexture("texture", this._texture, 0);
  }

  update() {}

  render() {
    GL.clear(0, 0, 0, 1);

    this._dAxis.draw();
    this._dDots.draw();

    this._drawSkybox.draw();
  }
}

export default SceneApp;
