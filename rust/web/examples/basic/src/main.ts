import init from "belfast-wasm";

import { cameraOrbit } from "./examples/camera-orbit";
import { coloredTriangle } from "./examples/colored-triangle";
import { computeTriangle } from "./examples/compute-triangle";
import { textureExample } from "./examples/texture";
import "../../src/style.css";

export type ReportError = (error: unknown) => void;

export type WebExample = (
  canvas: HTMLCanvasElement,
  reportError: ReportError,
) => Promise<() => void>;

const examples: Record<string, WebExample> = {
  "camera-orbit": cameraOrbit,
  "colored-triangle": coloredTriangle,
  "compute-triangle": computeTriangle,
  texture: textureExample,
};

const canvas = document.querySelector<HTMLCanvasElement>("canvas");

if (!canvas) {
  throw new Error("Example canvas is missing");
}

const exampleCanvas = canvas;

let cleanup: (() => void) | undefined;

const reportError: ReportError = (error) => {
  cleanup?.();
  cleanup = undefined;

  const output = document.querySelector<HTMLPreElement>("pre") ?? document.createElement("pre");
  output.textContent = error instanceof Error ? (error.stack ?? error.message) : String(error);
  document.body.append(output);
};

async function start() {
  try {
    await init();

    const exampleName =
      new URLSearchParams(window.location.search).get("example") ?? "colored-triangle";
    const example = examples[exampleName];
    if (!example) {
      throw new Error(`Unknown example: ${exampleName}`);
    }

    cleanup = await example(exampleCanvas, reportError);
  } catch (error) {
    reportError(error);
  }
}

void start();
