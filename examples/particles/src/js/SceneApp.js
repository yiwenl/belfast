import { GL, Scene, FboPingPong, DrawAxis, DrawDotsPlane } from "alfrid";
import Config from "./Config";

class SceneApp extends Scene {
  constructor() {
    super();

    this.orbitalControl.rx.value = this.orbitalControl.ry.value = 0.3;
    this.resize();

    console.log(GL);
  }

  _initTextures() {
    console.log("init textures");
    const { num } = Config;

    const oSettings = {
      type: GL.FLOAT,
      minFilter: GL.NEAREST,
      magFilter: GL.NEAREST,
    };

    this._fbo = new FboPingPong(num, num, oSettings, 4);
  }

  _initViews() {
    console.log("init views");

    this._dAxis = new DrawAxis();
    this._dDots = new DrawDotsPlane();

    console.log(GL.width, GL.height);
  }

  update() {}

  render() {
    GL.viewport(0, 0, window.innerWidth, window.innerHeight);
    GL.clear(0, 0, 0, 1);

    this._dAxis.draw();
    this._dDots.draw();
  }

  resize() {
    console.log("this._GL.aspectRatio", this._GL.aspectRatio);
    GL.setSize(window.innerWidth, window.innerHeight);
  }
}

export default SceneApp;
