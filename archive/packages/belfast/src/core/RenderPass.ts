export interface RenderPassOptions {
  clearColor?: GPUColor;
  loadOp?: GPULoadOp;
  storeOp?: GPUStoreOp;
  resolveTarget?: GPUTextureView;
  depthStencilAttachment?: GPURenderPassDepthStencilAttachment;
}

export interface RenderPassTarget {
  colorView: GPUTextureView;
  depthView?: GPUTextureView;
}

export function beginRenderPass(
  commandEncoder: GPUCommandEncoder,
  viewOrTarget: GPUTextureView | RenderPassTarget,
  options: RenderPassOptions = {},
): GPURenderPassEncoder {
  const {
    clearColor = { r: 0.05, g: 0.05, b: 0.08, a: 1 },
    loadOp = "clear",
    storeOp = "store",
    resolveTarget,
    depthStencilAttachment,
  } = options;

  const resolvedView = "colorView" in viewOrTarget ? viewOrTarget.colorView : viewOrTarget;
  const resolvedDepthAttachment =
    depthStencilAttachment ??
    ("colorView" in viewOrTarget && viewOrTarget.depthView
      ? {
          view: viewOrTarget.depthView,
          depthLoadOp: "clear",
          depthClearValue: 1,
          depthStoreOp: "store",
        }
      : undefined);

  return commandEncoder.beginRenderPass({
    colorAttachments: [
      {
        view: resolvedView,
        resolveTarget,
        clearValue: clearColor,
        loadOp,
        storeOp,
      },
    ],
    depthStencilAttachment: resolvedDepthAttachment,
  });
}
