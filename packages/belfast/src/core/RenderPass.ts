export interface RenderPassOptions {
  clearColor?: GPUColor;
  loadOp?: GPULoadOp;
  storeOp?: GPUStoreOp;
  depthStencilAttachment?: GPURenderPassDepthStencilAttachment;
}

export function beginRenderPass(
  commandEncoder: GPUCommandEncoder,
  view: GPUTextureView,
  options: RenderPassOptions = {},
): GPURenderPassEncoder {
  const {
    clearColor = { r: 0.05, g: 0.05, b: 0.08, a: 1 },
    loadOp = "clear",
    storeOp = "store",
    depthStencilAttachment,
  } = options;

  return commandEncoder.beginRenderPass({
    colorAttachments: [
      {
        view,
        clearValue: clearColor,
        loadOp,
        storeOp,
      },
    ],
    depthStencilAttachment,
  });
}
