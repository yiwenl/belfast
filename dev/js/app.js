import "../scss/global.scss";

import {
  GL,
  GLTool,
  Geom,
  CameraPerspective,
  DrawAxis,
  DrawDotsPlane,
  DrawCopy,
  DrawBall,
  DrawLine,
  OrbitalControl,
  Ray,
} from "../../src/alfrid";
import { vec3, mat4 } from "gl-matrix";
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
// const contexts = [GL];
console.log(GL);

const ray = new Ray([0, 0, 0], [1, 1, 1]);
contexts.forEach((_GL) => _init(_GL));

function _init(mGL) {
  // helpers
  const drawAxis = new DrawAxis(mGL);
  const drawDotsPlane = new DrawDotsPlane(mGL);
  const drawCopy = new DrawCopy(mGL);
  const drawBall = new DrawBall(mGL);
  const drawLine = new DrawLine(mGL);

  // camera
  const camera = new CameraPerspective(Math.PI / 2, GL.getAspectRatio(), 1, 10);
  const control = new OrbitalControl(camera, window, 5);

  // plane
  const mesh = Geom.plane(1, 1, 1);
  mesh.generateFaces();

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

    const t = vec3.clone(ray.origin);
    vec3.add(t, t, ray.direction);

    mGL.setMatrices(camera);
    drawAxis.draw();
    drawDotsPlane.draw();
    drawLine.draw(ray.origin, t, [1, 1, 0]);
  }

  // resize
  window.addEventListener("resize", resize);
  resize();

  function resize() {
    mGL.setSize(window.innerWidth / 2, window.innerHeight);
    camera.setAspectRatio(mGL.getAspectRatio());
  }
}
