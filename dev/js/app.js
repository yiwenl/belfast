import "../scss/global.scss";

import {
  GL,
  GLTool,
  CameraPerspective,
  GLTexture,
  Draw,
  DrawAxis,
  DrawDotsPlane,
  OrbitalControl,
  WebGLNumber,
} from "../../src/alfrid";
import { vec3, mat4 } from "gl-matrix";
import Scheduler from "scheduling";

import vs from "../shaders/test.vert";
import fs from "../shaders/test.frag";

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

GL.init(canvas1);
GL.setSize(window.innerWidth / 2, window.innerHeight);

const s = 10;
const mtx = mat4.create();
mat4.scale(mtx, mtx, [s, s, s]);

const GL2 = new GLTool();
const ctx2 = canvas2.getContext("webgl");
GL2.init(ctx2);
GL2.setSize(window.innerWidth / 2, window.innerHeight);
console.log(GL, GL2);

const contexts = [GL, GL2];

contexts.forEach((_GL) => _init(_GL));

function _init(mGL) {
  const positions = [[-1, 1, 0], [1, 1, 0], [-1, -1, 0], [1, -1, 0]];
  const uvs = [[0, 1], [1, 1], [0, 0], [1, 0]];
  const indices = [2, 1, 0, 2, 3, 1];

  const draw = new Draw(mGL);
  draw
    .useProgram(vs, fs)
    .createMesh()
    .bufferVertex(positions)
    .bufferTexCoord(uvs)
    .bufferIndex(indices);

  // helpers
  const drawAxis = new DrawAxis(mGL);
  const drawDotsPlane = new DrawDotsPlane(mGL);

  // camera
  const camera = new CameraPerspective(
    Math.PI / 2,
    GL.getAspectRatio(),
    0.1,
    100
  );

  camera.lookAt([2, 2, 5], [0, 0, 0], [0, 1, 0]);
  const control = new OrbitalControl(camera, window, 8);
  // control.rx.setTo(-1);

  const img = new Image();
  img.addEventListener("load", onImageLoaded);
  img.src = "./assets/img/test2.png";

  let texture;

  function onImageLoaded() {
    // data texture
    const data = [];
    const w = 32;
    const h = 32;
    const float32 = true;
    for (let i = 0; i < w; i++) {
      for (let j = 0; j < h; j++) {
        if (float32) {
          data.push(random(-1, 1));
          data.push(random(-1, 1));
          data.push(random(-1, 1));
          data.push(1);
        } else {
          data.push(randomFloor(256));
          data.push(randomFloor(256));
          data.push(randomFloor(256));
          data.push(255);
        }
      }
    }

    const source = float32 ? new Float32Array(data) : new Uint8Array(data);
    const oParams = {
      minFilter: mGL.NEAREST,
      magFilter: mGL.NEAREST,
    };
    if (float32) {
      oParams.type = mGL.FLOAT;
    }

    texture = new GLTexture(img);
    // texture = new GLTexture(source, oParams, w, h);
    draw.bindTexture("texture", texture, 0);
    Scheduler.addEF(() => render(mGL));

    setTimeout(() => {
      if (mGL.webgl2) {
        // texture.magFilter = mGL.LINEAR;
        // texture.minFilter = mGL.LINEAR;
      }
    }, 1000);
  }

  // render();
  function render(mGL) {
    const g = 0.1;
    if (mGL.webgl2) {
      mGL.clear(g, 0, 0, 1);
    } else {
      mGL.clear(0, g, 0, 1);
    }

    mGL.setMatrices(camera);
    drawAxis.draw();
    drawDotsPlane.draw();

    mGL.setModelMatrix(mtx);
    draw.draw();
  }
}

// resize
window.addEventListener("resize", resize);

function resize() {
  GL.setSize(window.innerWidth / 2, window.innerHeight);
  GL2.setSize(window.innerWidth / 2, window.innerHeight);
}
