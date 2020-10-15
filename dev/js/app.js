import "../scss/global.scss";

import { GL } from "../../src/alfrid";
const canvas = document.createElement("canvas");
GL.init(canvas);

document.body.appendChild(canvas);

GL.clear(0, 0, 0, 1);
