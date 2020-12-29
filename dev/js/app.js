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
import vsSave2 from "../shaders/save2.vert";
import fsSave from "../shaders/save.frag";
import fsSave2 from "../shaders/save2.frag";

import vsRender from "../shaders/render.vert";
import fsRender from "../shaders/render.frag";

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
console.log(GL);

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
  const fbo = new FrameBuffer(
    num,
    num,
    {
      minFilter: mGL.NEAREST,
      magFilter: mGL.NEAREST,
      type: mGL.FLOAT,
      mipmap: false,
    },
    2
  );

  // draw calls
  const meshSave = (() => {
    const positions = [];
    const uvs = [];
    const extras = [];
    const indices = [];
    const r = 1;
    let count = 0;

    for (let i = 0; i < num; i++) {
      for (let j = 0; j < num; j++) {
        const v = vec3.create();
        vec3.random(v, r);
        vec3.scale(v, v, Math.sqrt(Math.random()) * 3);
        positions.push(v);
        extras.push([Math.random(), Math.random(), Math.random()]);
        uvs.push([(i / num) * 2 - 1, (j / num) * 2 - 1]);
        indices.push(count);
        count++;
      }
    }

    const mesh = new Mesh(mGL.POINTS)
      .bufferVertex(positions)
      .bufferData(extras, "aExtra")
      .bufferTexCoord(uvs)
      .bufferIndex(indices);

    return mesh;
  })();

  const drawSave = new Draw(mGL)
    .setMesh(meshSave)
    .useProgram(mGL.webgl2 ? vsSave2 : vsSave, mGL.webgl2 ? fsSave2 : fsSave)
    .setClearColor(0, 0, 0, 1)
    .bindFrameBuffer(fbo)
    .draw();

  const meshRender = (() => {
    const positions = [];
    const indices = [];
    let count = 0;

    for (let i = 0; i < num; i++) {
      for (let j = 0; j < num; j++) {
        positions.push([i / num, j / num, Math.random()]);
        indices.push(count);
        count++;
      }
    }

    const mesh = new Mesh(mGL.POINTS)
      .bufferVertex(positions)
      .bufferIndex(indices);

    return mesh;
  })();

  const drawRender = new Draw(mGL)
    .setMesh(meshRender)
    .useProgram(vsRender, fsRender);

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

    drawRender
      .bindTexture("texturePos", fbo.texture, 0)
      .uniform("uViewport", [mGL.width, mGL.height])
      .draw();

    const s = num;
    mGL.viewport(0, 0, s, s);
    drawCopy.draw(fbo.texture);
    mGL.viewport(s, 0, s, s);
    drawCopy.draw(fbo.getTexture(1));
  }

  // resize
  window.addEventListener("resize", resize);
  resize();

  function resize() {
    mGL.setSize(window.innerWidth / 2, window.innerHeight);
    camera.setAspectRatio(mGL.getAspectRatio());
  }
}
