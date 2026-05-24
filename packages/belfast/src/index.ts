export { Device, type DeviceOptions } from "./core/Device";
export { beginRenderPass, type RenderPassOptions } from "./core/RenderPass";
export { Draw, type DrawOptions } from "./helper/Draw";

import { Device } from "./core/Device";

export function showWebGPUUnavailableMessage(container: ParentNode = document.body): void {
  const message = document.createElement("div");
  message.style.cssText =
    "position:fixed;inset:0;display:flex;align-items:center;justify-content:center;padding:2rem;font:16px/1.5 system-ui,sans-serif;background:#111;color:#eee;text-align:center;";
  message.textContent =
    "WebGPU is not available in this browser. Try the latest Chrome, Edge, or Safari.";
  container.appendChild(message);
}

export async function assertWebGPUSupport(): Promise<void> {
  if (!(await Device.isSupported())) {
    showWebGPUUnavailableMessage();
    throw new Error("WebGPU is not supported.");
  }
}
