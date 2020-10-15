import "../scss/global.scss";

import { GL } from "../../src/alfrid";
const canvas = document.createElement("canvas");
GL.init(canvas, { webgl2: true });

document.body.appendChild(canvas);
