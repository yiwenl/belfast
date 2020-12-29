import "../scss/global.scss";

import {
  GL,
  GLTool,
  CameraPerspective,
  GLTexture,
  Draw,
  DrawAxis,
  DrawDotsPlane,
  DrawCopy,
  DrawBall,
  OrbitalControl,
  FrameBuffer,
  Geom,
  Object3D,
  WebGLNumber,
} from "../../src/alfrid";
import { vec3, mat4 } from "gl-matrix";
import Scheduler from "scheduling";

import vs from "../shaders/test.vert";
import fs from "../shaders/test.frag";
import fsCopy from "../shaders/copy.frag";

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

const s = 1;
const mtx = mat4.create();
mat4.scale(mtx, mtx, [s, s, s]);
mat4.translate(mtx, mtx, [1, 0, 0]);

const GL2 = new GLTool();
const ctx2 = canvas2.getContext("webgl");
GL2.init(ctx2);
GL2.setSize(window.innerWidth / 2, window.innerHeight);

const contexts = [GL, GL2];

contexts.forEach((_GL) => _init(_GL));

const container = new Object3D();

function _init(mGL) {
  mGL.enableAlphaBlending();
  let s = 2;
  const draw = new Draw(mGL);
  // draw.useProgram(vs, fs).setMesh(Geom.plane(s, s, 1));
  draw.useProgram(vs, fs).setMesh(Geom.cube(s));

  // helpers
  const drawAxis = new DrawAxis(mGL);
  const drawDotsPlane = new DrawDotsPlane(mGL);
  const drawCopy = new DrawCopy(mGL);
  const drawBall = new DrawBall(mGL);

  s = 4;
  const drawDebug = new Draw(mGL)
    .useProgram(vs, fsCopy)
    .setMesh(Geom.plane(s, s, 1));

  // camera
  const camera = new CameraPerspective(Math.PI / 2, GL.getAspectRatio(), 1, 10);

  camera.lookAt([2, 2, 5], [0, 0, 0], [0, 1, 0]);
  const control = new OrbitalControl(camera, window, 8);
  // control.rx.setTo(-1);

  const fboSize = 1024;
  const fbo = new FrameBuffer(fboSize, fboSize);
  console.log(fbo);

  const img = new Image();
  img.addEventListener("load", onImageLoaded);
  img.src = "./assets/img/test1.jpg";

  let texture, textureImg;

  function onImageLoaded() {
    // data texture
    const data = [];
    const w = 64;
    const h = 64;
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
      minFilter: GL.NEAREST,
      magFilter: GL.NEAREST,
    };
    if (float32) {
      oParams.type = mGL.FLOAT;
    }

    // texture = new GLTexture(img, oParams);
    texture = new GLTexture(source, oParams, w, h);
    textureImg = new GLTexture(img);
    draw.bindTexture("texture", textureImg, 0);

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
    container.rotationX = Scheduler.deltaTime;
    container.rotationY = -Scheduler.deltaTime;
    container.rotationZ = Scheduler.deltaTime * 0.3;

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

    fbo.bind(mGL);
    mGL.clear(0, 0, 0, 0);
    mGL.setModelMatrix(container.matrix);
    draw.draw();
    // drawCopy.draw(texture);
    fbo.unbind();

    mGL.setModelMatrix(mat4.create());

    let s = 0.2;
    drawBall.draw([-1, 1, 0], [s, s, s], [1, 0, 0]);
    drawBall.draw([1, 1, 0], [s, s, s], [0, 1, 0]);
    drawBall.draw([-1, -1, 0], [s, s, s], [0, 0, 1]);
    drawBall.draw([1, -1, 0], [s, s, s], [1, 1, 0]);

    drawDebug.bindTexture("texture", fbo.texture, 0).draw();

    s = 100;
    mGL.viewport(0, 0, s, s);
    drawCopy.draw(texture);
    mGL.viewport(s, 0, s, s);
    drawCopy.draw(textureImg);
    mGL.viewport(s * 2, 0, s, s);
    drawCopy.draw(fbo.depthTexture);
    mGL.enable(mGL.DEPTH_TEST);
    // mGL.viewport(s, 0, s, s);
    // drawCopy.draw(fbo.texture);
  }
  // resize
  window.addEventListener("resize", resize);
  resize();

  function resize() {
    mGL.setSize(window.innerWidth / 2, window.innerHeight);
    camera.setAspectRatio(mGL.getAspectRatio());
  }
}
