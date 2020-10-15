import "../scss/global.scss";

import { GL, GLTool } from "../../src/alfrid";
const canvas1 = document.createElement("canvas");
const canvas2 = document.createElement("canvas");
GL.init(canvas1);
GL.setSize(window.innerWidth / 2, window.innerHeight);

const GL2 = new GLTool();
GL2.init(canvas2);
GL2.setSize(window.innerWidth / 2, window.innerHeight);

document.body.appendChild(canvas1);
document.body.appendChild(canvas2);

console.log("ID:", GL.id, GL2.id);

GL.clear(0.5, 0, 0, 1);
GL2.clear(0, 0.5, 0, 1);
