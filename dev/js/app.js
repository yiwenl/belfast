import "../scss/global.scss";

import { GL, GLTool, GLShader, Mesh } from "../../src/alfrid";
const canvas1 = document.createElement("canvas");
const canvas2 = document.createElement("canvas");
GL.init(canvas1);
GL.setSize(window.innerWidth / 2, window.innerHeight);

const GL2 = new GLTool();
GL2.init(canvas2);
GL2.setSize(window.innerWidth / 2, window.innerHeight);

document.body.appendChild(canvas1);
document.body.appendChild(canvas2);

console.log("ID 1:", GL.id, "ID 2:", GL2.id);

const g = 0.1;
GL.clear(g, 0, 0, 1);
GL2.clear(0, g, 0, 1);

const draw1 = false;
const shader = new GLShader();
const mesh = new Mesh();

const s = 0.5;
const positions = [[0, s, 0], [s, -s / 2, 0], [-s, -s / 2, 0]];
// const colors = [[1, 0, 0], [0, 1, 0], [0, 0, 1]];
const indices = [0, 1, 2];
mesh.bufferVertex(positions).bufferIndex(indices);

shader.uniform("uTime", 0.1);
shader.uniform("uCenters", [0.25, 0.25, 0.75, 0.75], "vec2");

if (draw1) {
  shader.bind();
  GL.draw(mesh);
} else {
  shader.bind(GL2);
  GL2.draw(mesh);
}
