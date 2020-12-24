import "../scss/global.scss";

import {
  GL,
  GLTool,
  GLShader,
  Mesh,
  CameraPerspective,
  Draw,
  DrawAxis,
} from "../../src/alfrid";
import { vec3, mat4, mat3 } from "gl-matrix";
import Scheduler from "scheduling";

import vs from "../shaders/test.vert";
import fs from "../shaders/test.frag";

const canvas1 = document.createElement("canvas");
const canvas2 = document.createElement("canvas");

GL.init(canvas1);
GL.setSize(window.innerWidth / 2, window.innerHeight);

const GL2 = new GLTool();
const ctx2 = canvas2.getContext("webgl");
GL2.init(ctx2);
GL2.setSize(window.innerWidth / 2, window.innerHeight);

document.body.appendChild(canvas1);
document.body.appendChild(canvas2);

const draw1 = Math.random() > 0.5;
const shader = new GLShader(vs, fs);
const mesh = new Mesh();

let s = 0.5;
const positions = [
  vec3.fromValues(0, s, 0),
  vec3.fromValues(-s, -s / 2, 0),
  vec3.fromValues(s, -s / 2, 0),
];
const colors = [[0, 0, 0], [1, 1, 0], [2, 0, 1]];
const indices = [0, 1, 2];

mesh
  .bufferVertex(positions)
  .bufferData(colors, "aColor")
  .bufferIndex(indices);

let draw;
if (draw1) {
  draw = new Draw();
} else {
  draw = new Draw(GL2);
}

draw.setMesh(mesh).useProgram(shader);

const drawAxis = new DrawAxis(draw1 ? GL : GL2);

// uniforms

const g = 0.5;
const mtx = mat4.create();

const camera = new CameraPerspective(
  Math.PI / 2,
  GL.getAspectRatio(),
  0.1,
  100
);

camera.lookAt([2, 2, 5], [0, 0, 0], [0, 1, 0]);
console.log(camera);
Scheduler.addEF(render);
// render();

function render() {
  let g = 0.1;
  GL.clear(g, 0, 0, 1);
  GL2.clear(0, g, 0, 1);

  /*
   * Shader and Mesh ( buffers ) won't be created until they are going to be bind
   * They'll be generated by the WebGL context that it binds or draws
   */
  if (draw1) {
    GL.setMatrices(camera);
    // shader.bind(GL);
    // GL.draw(mesh);
    draw.draw();
    drawAxis.draw();
  } else {
    GL2.setMatrices(camera);
    // shader.bind(GL2);
    // GL2.draw(mesh);
    draw.draw();
    drawAxis.draw();
  }
}

// resize
window.addEventListener("resize", resize);

function resize() {
  GL.setSize(window.innerWidth / 2, window.innerHeight);
  GL2.setSize(window.innerWidth / 2, window.innerHeight);
}
