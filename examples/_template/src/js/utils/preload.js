// preload.js
import assets from "../asset-list";
import AssetsLoader from "assets-loader";

const loadAssets = () =>
  new Promise((resolve, reject) => {
    console.log("Load Assets", assets);
    if (assets.length > 0) {
      document.body.classList.add("isLoading");

      new AssetsLoader({
        assets: assets,
      })
        .on("error", (error) => {
          console.log("Error :", error);
        })
        .on("progress", (p) => {
          // console.log('Progress : ', p);
          // const loader = document.body.querySelector(".Loading-Bar");
          // if (loader) loader.style.width = `${p * 100}%`;
        })
        .on("complete", (o) => {
          resolve(o);
        })
        .start();
    } else {
      resolve([]);
    }
  });

export default loadAssets;