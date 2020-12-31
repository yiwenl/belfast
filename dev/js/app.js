import "../scss/global.scss";

import {
  GL,
  GLTool,
  Geom,
  CameraPerspective,
  Draw,
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

import vs from "../shaders/test.vert";
import fs from "../shaders/test.frag";

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
  control.rx.value = control.ry.value = 0.3;

  const camera1 = new CameraPerspective(
    Math.PI / 2,
    GL.getAspectRatio(),
    1,
    10
  );
  camera1.lookAt([2, 2, 2], [0, 1, 0]);

  const ray = camera1.generateRay([0, 0, 0]);
  console.log(ray.origin, ray.direction);

  // plane
  const s = 5;
  const mesh = Geom.plane(s, s, 1);
  mesh.generateFaces();

  const draw = new Draw(mGL).setMesh(mesh).useProgram(vs, fs);
  const hit = vec3.create();

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

    const s = 0.1;
    drawBall.draw(hit, [s, s, s], [1, 1, 1]);
    // drawBall.draw(camera1.position, [s, s, s], [0, 0, 1]);
    drawBall.draw(ray.origin, [s, s, s], [0, 0, 1]);

    draw.draw();
  }

  // resize
  window.addEventListener("resize", resize);
  resize();

  function resize() {
    mGL.setSize(window.innerWidth / 2, window.innerHeight);
    camera.setAspectRatio(mGL.getAspectRatio());
  }
}
