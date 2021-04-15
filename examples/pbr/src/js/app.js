import "../scss/global.scss";

import "./utils/Capture";
import { GL } from "alfrid";

import Settings from "./Settings";
import SceneApp from "./SceneApp";
import preload from "./utils/preload";
import Assets from "./Assets";
import addControls from "./debug/addControls";

let scene;

if (document.body) {
  _init();
} else {
  window.addEventListener("DOMContentLoaded", _init);
}

function _init() {
  preload().then(_init3D, (e) => {
    console.error(e);
  });
}

function _init3D(o) {
  const canvas = document.createElement("canvas");
  canvas.width = window.innerWidth;
  canvas.height = window.innerHeight;

  document.body.appendChild(canvas);
  canvas.className = "Main-Canvas";
  const preserveDrawingBuffer = process.env.NODE_ENV === "development";
  const webgl1 = false;
  GL.init(canvas, { webgl1, preserveDrawingBuffer });
  Assets.init(o);

  if (process.env.NODE_ENV === "development") {
    Settings.init();
  }

  scene = new SceneApp();

  if (process.env.NODE_ENV === "development") {
    addControls(scene);
  }
}
