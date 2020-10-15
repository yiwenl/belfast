import "../scss/global.scss";

import { GL, GLTool, GLShader } from "../../src/alfrid";
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

GL.clear(0.5, 0, 0, 1);
GL2.clear(0, 0.5, 0, 1);

const draw1 = true;
const shader = new GLShader();

shader.uniform("uTime", 0.1);
shader.uniform("uCenters", [0.25, 0.25, 0.75, 0.75], "vec2");

if (draw1) {
  shader.bind();
} else {
  shader.bind(GL2);
}

console.log("Shader Count, 1:", GL.shaderCount, "2:", GL2.shaderCount);
