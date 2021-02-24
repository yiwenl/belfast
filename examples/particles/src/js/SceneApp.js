import {
  GL,
  Scene,
  FboPingPong,
  Draw,
  DrawAxis,
  DrawDotsPlane,
  DrawCopy,
  DrawCamera,
  FrameBuffer,
  CameraOrtho,
  Geom,
  getColorTexture
} from "alfrid";
import Config from "./Config";
import Scheduler from "scheduling";
import { biasMatrix } from './utils'
import { mat4 } from 'gl-matrix'

// draw calls
import DrawSave from "./DrawSave";
import DrawRender from "./DrawRender";

// shaders
import vsSim from "shaders/sim.vert";
import fsSim from "shaders/sim.frag";

class SceneApp extends Scene {
  constructor() {
    super();

    // this.orbitalControl.rx.value = this.orbitalControl.ry.value = 0.3;
    this.orbitalControl.radius.value = 15;

    // shadow
    this.light = [1.2, 5, 1.2]
    const r = 5.5;
    this.cameraLight = new CameraOrtho();
    this.cameraLight.ortho(-r, r, r, -r, .5, 10);
    this.cameraLight.lookAt(this.light, [0, 0, 0])

    this._mtxShadow = mat4.create()
    mat4.mul(this._mtxShadow, this.cameraLight.projection, this.cameraLight.view)
    mat4.mul(this._mtxShadow, biasMatrix, this._mtxShadow)

    this.resize();
  }

  _initTextures() {
    // console.log("init textures");
    const { num } = Config;

    const oSettings = {
      type: GL.FLOAT,
      minFilter: GL.NEAREST,
      magFilter: GL.NEAREST,
      mipmap: false,
    };

    this._fbo = new FboPingPong(num, num, oSettings, 4);

    const shadowMapSize = 2048;
    this._fboShadow = new FrameBuffer(shadowMapSize, shadowMapSize)
    
    this._textureWhite = getColorTexture([1, 1, 1])
  }

  _initViews() {
    console.log("init views");

    this._dAxis = new DrawAxis();
    this._dDots = new DrawDotsPlane();
    this._dCopy = new DrawCopy();
    this._dCamera = new DrawCamera()

    const drawSave = new DrawSave();
    drawSave.bindFrameBuffer(this._fbo.read).draw();

    this._drawRender = new DrawRender();
    this._drawSim = new Draw()
      .setMesh(Geom.bigTriangle())
      .useProgram(vsSim, fsSim);
  }

  update() {
    this._drawSim
      .bindFrameBuffer(this._fbo.write)
      .bindTexture("texturePos", this._fbo.read.getTexture(0), 0)
      .bindTexture("textureVel", this._fbo.read.getTexture(1), 1)
      .bindTexture("textureExtra", this._fbo.read.getTexture(2), 2)
      .bindTexture("textureLife", this._fbo.read.getTexture(3), 3)
      .uniform("uTime", Scheduler.deltaTime)
      .draw();

    this._fbo.swap();

    GL.setMatrices(this.cameraLight)
    this._fboShadow.bind();
    GL.clear(0, 0, 0, 0);
    this.renderParticles(false);
    this._fboShadow.unbind();
  }

  renderParticles(mShadow) {
    const tDepth = mShadow ? this._fboShadow.depthTexture : this._textureWhite;
    
    this._drawRender
      .uniform("uViewport", [GL.width, GL.height])
      .uniform("uShadowMatrix", this._mtxShadow)
      .bindTexture("texturePos", this._fbo.read.getTexture(0), 0)
      .bindTexture("textureDepth", tDepth, 1)
      .draw();
  }

  render() {
    const g = .1;
    GL.viewport(0, 0, window.innerWidth, window.innerHeight);
    GL.clear(g, g, g, 1);

    this._dAxis.draw();
    this._dDots.draw();
    this._dCamera.draw(this.cameraLight)

    this.renderParticles(true);

    const s = GL.isMobile ? 64 : 128;
    GL.viewport(0, 0, s, s);
    this._dCopy.draw(this._fbo.read.getTexture(0));
    GL.viewport(s, 0, s, s);
    this._dCopy.draw(this._fboShadow.depthTexture);
  }
}

export default SceneApp;
