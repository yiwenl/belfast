import "../scss/global.scss";

import {
  GL,
  GLTool,
  Mesh,
  CameraPerspective,
  Draw,
  DrawAxis,
  DrawDotsPlane,
  DrawCopy,
  DrawBall,
  OrbitalControl,
  FrameBuffer,
} from "../../src/alfrid";
import { vec3, mat4 } from "gl-matrix";
import Scheduler from "scheduling";

import vsSave from "../shaders/save.vert";
import fsSave from "../shaders/save.frag";

const randomFloor = (v) => {
  return Math.floor(Math.random() * v);
};

const random = (a, b) => {
  return a + Math.random() * (b - a);
};

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

contexts.forEach((_GL) => _init(_GL));

function _init(mGL) {
  // helpers
  const drawAxis = new DrawAxis(mGL);
  const drawDotsPlane = new DrawDotsPlane(mGL);
  const drawCopy = new DrawCopy(mGL);
  const drawBall = new DrawBall(mGL);

  // camera
  const camera = new CameraPerspective(Math.PI / 2, GL.getAspectRatio(), 1, 10);
  const control = new OrbitalControl(camera, window, 5);

  // fbo
  const num = 128;
  const fbo = new FrameBuffer(num, num, {
    minFilter: mGL.NEAREST,
    magFilter: mGL.NEAREST,
    type: mGL.FLOAT,
    mipmap: false,
  });

  // draw

  const meshSave = (() => {
    const positions = [];
    const uvs = [];
    const indices = [];
    const r = 4;
    let count = 0;

    for (let i = 0; i < num; i++) {
      for (let j = 0; j < num; j++) {
        const v = vec3.create();
        vec3.random(v, r);
        positions.push(v);
        uvs.push([(i / num) * 2 - 1, (j / num) * 2 - 1]);
        indices.push(count);
        count++;
      }
    }

    const mesh = new Mesh(mGL.POINTS)
      .bufferVertex(positions)
      .bufferTexCoord(uvs)
      .bufferIndex(indices);

    return mesh;
  })();

  const drawSave = new Draw(mGL)
    .setMesh(meshSave)
    .useProgram(vsSave, fsSave)
    .setClearColor(0, 0, 0, 1)
    .bindFrameBuffer(fbo)
    .draw();

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

    mGL.viewport(0, 0, num, num);
    drawCopy.draw(fbo.texture);
  }

  // resize
  window.addEventListener("resize", resize);
  resize();

  function resize() {
    mGL.setSize(window.innerWidth / 2, window.innerHeight);
    camera.setAspectRatio(mGL.getAspectRatio());
  }
}
