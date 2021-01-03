import {
  GL,
  Scene,
  Draw,
  DrawAxis,
  DrawDotsPlane,
  DrawCopy,
  parseObj,
  loadObj,
} from "alfrid";

import Assets from "./Assets";
import { mat4 } from "gl-matrix";

// shader
import vs from "shaders/basic.vert";
import fs from "shaders/diffuse.frag";

class SceneApp extends Scene {
  constructor() {
    super();

    this.mtx = mat4.create();
    mat4.translate(this.mtx, this.mtx, [0, 0, -2]);

    this.orbitalControl.rx.value = this.orbitalControl.ry.value = 0.3;
    this.orbitalControl.radius.value = 15;
    this.resize();
  }

  _initTextures() {}

  _initViews() {
    this._dAxis = new DrawAxis();
    this._dDots = new DrawDotsPlane();
    this._dCopy = new DrawCopy();

    // const oModel = Assets.get("bike");
    const oModel = Assets.get("bikeParts");
    const meshes = parseObj(oModel);

    loadObj("assets/obj/giant.obj").then(
      (meshGiant) => {
        this.drawGiant = new Draw().setMesh(meshGiant).useProgram(vs, fs);
      },
      (err) => console.log(err)
    );

    this.draw = new Draw().setMesh(meshes).useProgram(vs, fs);
  }

  render() {
    GL.clear(0, 0, 0, 1);

    this._dAxis.draw();
    this._dDots.draw();

    this.draw.draw();

    if (this.drawGiant) {
      GL.setModelMatrix(this.mtx);
      this.drawGiant.draw();
    }
  }
}

export default SceneApp;
