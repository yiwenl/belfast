// addControls.js

import Settings from "../Settings";
import Config from "../Config";
import { saveJson } from "../utils";
import { GL } from "alfrid";

const addControls = (scene) => {
  const oControl = {
    save: () => {
      saveJson(Config, "Settings");
    },
    webgl2: GL.webgl2.toString(),
  };

  gui.add(oControl, "webgl2");

  setTimeout(() => {
    gui
      .add(Config, "env", ["street", "skyfire"])
      .onFinishChange(Settings.reload);
    gui.addColor(Config, "color").onFinishChange(Settings.refresh);
    gui.add(Config, "roughness", 0, 1).onFinishChange(Settings.refresh);
    gui.add(Config, "metallic", 0, 1).onFinishChange(Settings.refresh);
    gui.add(Config, "exposure", 0, 3).onFinishChange(Settings.refresh);
    gui.add(oControl, "save").name("Save Settings");
    gui.add(Settings, "reset").name("Reset Default");
  }, 200);
};

export default addControls;
