import "../scss/global.scss";

import {
  GL,
  GLTool,
  Geom,
  CameraPerspective,
  Draw,
  DrawAxis,
  DrawDotsPlane,
  DrawBall,
  DrawCamera,
  OrbitalControl,
  DiffuseLightShader,
} from "../../src/alfrid";
import Scheduler from "scheduling";

const canvas1 = document.createElement("canvas");
const canvas2 = document.createElement("canvas");
document.body.appendChild(canvas1);
document.body.appendChild(canvas2);

const webgl1 = false;
GL.init(canvas1, { webgl1 });
GL.setSize(window.innerWidth / 2, window.innerHeight);

const GL2 = new GLTool();
const ctx2 = canvas2.getContext("webgl");
GL2.init(ctx2);
GL2.setSize(window.innerWidth / 2, window.innerHeight);

const contexts = [GL, GL2];
console.log(GL);

contexts.forEach((_GL) => _init(_GL));

function _init(mGL) {
  // helpers
  const drawAxis = new DrawAxis(mGL);
  const drawDotsPlane = new DrawDotsPlane(mGL);
  const drawBall = new DrawBall(mGL);
  const drawCamera = new DrawCamera(mGL);

  // test
  const shaderDiffuse = new DiffuseLightShader();
  shaderDiffuse.color = [Math.random(), Math.random(), Math.random()];
  shaderDiffuse.intensity = Math.random() * 0.5 + 0.25;
  const drawTest = new Draw(mGL)
    .setMesh(Geom.sphere(1, 24))
    .useProgram(shaderDiffuse);

  // camera
  const camera = new CameraPerspective(Math.PI / 2, GL.getAspectRatio(), 1, 50);
  const camera1 = new CameraPerspective(Math.PI / 2, GL.getAspectRatio(), 1, 5);
  const control = new OrbitalControl(camera, window, 5);
  control.rx.value = control.ry.value = 0.3;
  camera1.lookAt([2, 2, 2], [0, 0, 0]);

  Scheduler.addEF(() => render(mGL));

  // render();
  function render(mGL) {
    mGL.viewport(0, 0, mGL.width, mGL.height);
    const g = 0.1;
    if (mGL.webgl2) {
      mGL.clear(g, 0, 0, 1);
    } else {
      mGL.clear(0, g, 0, 1);
    }

    mGL.setMatrices(camera);
    drawAxis.draw();
    drawDotsPlane.draw();
    drawTest.draw();

    const s = 0.1;
    drawCamera.draw(camera1, [1, 0.5, 0]);
    drawBall.draw([2, 2, 2], [s, s, s], [1, 0, 0]);
  }

  // resize
  window.addEventListener("resize", resize);
  resize();

  function resize() {
    mGL.setSize(window.innerWidth / 2, window.innerHeight);
    camera.setAspectRatio(mGL.getAspectRatio());
  }
}
