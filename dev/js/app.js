import "../scss/global.scss";

import { GL, GLTool } from "../../src/alfrid";

/*
// class test
import Test1 from "./test1";
import Test2 from "./test2";

const test1 = new Test1();
const test2 = new Test2();
console.log(test1);
console.log(test2);

test1.test();
test2.test();
*/

const canvas1 = document.createElement("canvas");
const canvas2 = document.createElement("canvas");

// GL.init(canvas1, { alpha: false });
GL.init(canvas1);
GL.setSize(window.innerWidth / 2, window.innerHeight);

const GL2 = new GLTool();
const ctx2 = canvas2.getContext("webgl", {
  alpha: true,
  depth: true,
  premultipliedAlpha: false,
});
GL2.init(ctx2);
GL2.setSize(window.innerWidth / 2, window.innerHeight);

document.body.appendChild(canvas1);
document.body.appendChild(canvas2);

const g = 0.1;
GL.clear(1, 0, 0, g);
GL2.clear(0, 1, 0, g);
/*

const draw1 = false;
const shader = new GLShader();
const mesh = new Mesh();

const s = 0.5;
const positions = [[0, s, 0], [s, -s / 2, 0], [-s, -s / 2, 0]];
const indices = [0, 1, 2];
mesh.bufferVertex(positions).bufferIndex(indices);

if (draw1) {
  shader.bind();
  GL.draw(mesh);
} else {
  shader.bind(GL2);
  GL2.draw(mesh);
}

mesh.destroy();
*/

// resize
window.addEventListener("resize", resize);

function resize() {
  GL.setSize(window.innerWidth / 2, window.innerHeight);
  GL2.setSize(window.innerWidth / 2, window.innerHeight);
}
