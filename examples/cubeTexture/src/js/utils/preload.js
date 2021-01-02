// preload.js
import assets from "../asset-list";
import Assets from "../Assets";
import AssetsLoader from "assets-loader";

const loadAssets = () =>
  new Promise((resolve, reject) => {
    const loader = document.body.querySelector(".Loading-Bar");
    console.log("Load Assets");
    if (assets.length > 0) {
      document.body.classList.add("isLoading");

      new AssetsLoader({
        assets: assets,
      })
        .on("error", (error) => {
          console.log("Error :", error);
        })
        .on("progress", (p) => {
          console.log("Progress : ", p);

          if (loader) loader.style.width = `${p * 100}%`;
        })
        .on("complete", (o) => {
          if (loader) loader.style.width = `100%`;
          document.body.classList.remove("isLoading");
          Assets.init(o);
          resolve(o);
        })
        .start();
    } else {
      resolve([]);
    }
  });

export default loadAssets;
