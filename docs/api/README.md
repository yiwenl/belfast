# API reference

Public exports from [`packages/belfast/src/index.ts`](../../packages/belfast/src/index.ts).

```ts
import {
  BindGroup,
  Buffer,
  UniformBlock,
  Texture,
  RenderTarget,
  Camera,
  Device,
  AxisHelper,
  BallHelper,
  CopyHelper,
  Geom,
  Draw,
  Compute,
  Mesh,
  PerspectiveCamera,
  OrbitalControl,
  EaseNumber,
  beginRenderPass,
  assertWebGPUSupport,
  showWebGPUUnavailableMessage,
} from "belfast";
```

## Modules

| Export                              | Kind     | Doc                                            |
| ----------------------------------- | -------- | ---------------------------------------------- |
| `Device`                            | class    | [Device.md](Device.md)                         |
| `DeviceOptions`                     | type     | [Device.md](Device.md)                         |
| `Buffer`                            | class    | [Buffer.md](Buffer.md)                         |
| `BufferUsage`                       | const    | [Buffer.md](Buffer.md)                         |
| `UniformBlock`                      | class    | [UniformBlock.md](UniformBlock.md)             |
| `UniformBlockSchema`                | type     | [UniformBlock.md](UniformBlock.md)             |
| `UniformFieldType`                  | type     | [UniformBlock.md](UniformBlock.md)             |
| `BindGroup`                         | class    | [BindGroup.md](BindGroup.md)                   |
| `BindGroupResource`                 | type     | [BindGroup.md](BindGroup.md)                   |
| `Texture`                           | class    | [Texture.md](Texture.md)                       |
| `TextureOptions`                    | type     | [Texture.md](Texture.md)                       |
| `createSceneTexturePipelineLayout`  | function | [Texture.md](Texture.md)                       |
| `createSceneTextureBindGroupLayout` | function | [Texture.md](Texture.md)                       |
| `createPlaneTriangleList`           | function | [Texture.md](Texture.md)                       |
| `PlaneAxis`                         | type     | [Texture.md](Texture.md)                       |
| `RenderTarget`                      | class    | [RenderTarget.md](RenderTarget.md)             |
| `RenderTargetOptions`               | type     | [RenderTarget.md](RenderTarget.md)             |
| `Mesh`                              | class    | [Mesh.md](Mesh.md)                             |
| `MeshIndexFormat`                   | type     | [Mesh.md](Mesh.md)                             |
| `VertexBufferBinding`               | type     | [Mesh.md](Mesh.md)                             |
| `VertexAttributeDescriptor`         | type     | [Mesh.md](Mesh.md)                             |
| `Geom`                              | class    | [Geom.md](Geom.md)                             |
| `GeometryData`                      | type     | [Geom.md](Geom.md)                             |
| `PlaneOptions`                      | type     | [Geom.md](Geom.md)                             |
| `SphereOptions`                     | type     | [Geom.md](Geom.md)                             |
| `CubeOptions`                       | type     | [Geom.md](Geom.md)                             |
| `Camera`                            | class    | [Camera.md](Camera.md)                         |
| `PerspectiveCamera`                 | class    | [PerspectiveCamera.md](PerspectiveCamera.md)   |
| `OrthographicCamera`                | class    | [OrthographicCamera.md](OrthographicCamera.md) |
| `Vec3`                              | type     | [Camera.md](Camera.md)                         |
| `MutVec3`                           | type     | [Camera.md](Camera.md)                         |
| `Mat4`                              | type     | [Camera.md](Camera.md)                         |
| `EaseNumber`                        | class    | [EaseNumber.md](EaseNumber.md)                 |
| `OrbitalControl`                    | class    | [OrbitalControl.md](OrbitalControl.md)         |
| `OrbitalControlOptions`             | type     | [OrbitalControl.md](OrbitalControl.md)         |
| `AxisHelper`                        | class    | [AxisHelper.md](AxisHelper.md)                 |
| `AxisHelperOptions`                 | type     | [AxisHelper.md](AxisHelper.md)                 |
| `createSceneUniformPipelineLayout`  | function | [AxisHelper.md](AxisHelper.md)                 |
| `createSceneUniformBindGroupLayout` | function | [AxisHelper.md](AxisHelper.md)                 |
| `BallHelper`                        | class    | [BallHelper.md](BallHelper.md)                 |
| `BallHelperOptions`                 | type     | [BallHelper.md](BallHelper.md)                 |
| `BallDrawParams`                    | type     | [BallHelper.md](BallHelper.md)                 |
| `createSceneBallPipelineLayout`     | function | [BallHelper.md](BallHelper.md)                 |
| `createBallInstanceBindGroupLayout` | function | [BallHelper.md](BallHelper.md)                 |
| `CopyHelper`                        | class    | [CopyHelper.md](CopyHelper.md)                 |
| `CopyHelperOptions`                 | type     | [CopyHelper.md](CopyHelper.md)                 |
| `Draw`                              | class    | [Draw.md](Draw.md)                             |
| `DrawOptions`                       | type     | [Draw.md](Draw.md)                             |
| `Compute`                           | class    | [Compute.md](Compute.md)                       |
| `ComputeOptions`                    | type     | [Compute.md](Compute.md)                       |
| `WorkgroupCount`                    | type     | [Compute.md](Compute.md)                       |
| `beginRenderPass`                   | function | [RenderPass.md](RenderPass.md)                 |
| `RenderPassOptions`                 | type     | [RenderPass.md](RenderPass.md)                 |
| `assertWebGPUSupport`               | function | [utilities.md](utilities.md)                   |
| `showWebGPUUnavailableMessage`      | function | [utilities.md](utilities.md)                   |

## Versioning

The package is early (`0.1.x`). Breaking changes are expected until the API stabilizes.
