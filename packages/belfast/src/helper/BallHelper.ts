import { BindGroup } from "../core/BindGroup";
import type { Device } from "../core/Device";
import { Buffer, BufferUsage } from "../core/Buffer";
import { Mesh } from "../core/Mesh";
import type { Vec3 } from "../math/types";
import { createSphereTriangleList } from "../geom/sphere";
import { Draw } from "./Draw";
import { createSceneBallPipelineLayout } from "./sceneLayout";

const BALL_SHADER = /* wgsl */ `
struct SceneUniforms {
  viewProj: mat4x4<f32>,
}

struct BallInstance {
  translate: vec3<f32>,
  _pad0: f32,
  scale: vec3<f32>,
  _pad1: f32,
  color: vec3<f32>,
  opacity: f32,
}

@group(0) @binding(0) var<uniform> scene: SceneUniforms;
@group(1) @binding(0) var<uniform> ball: BallInstance;

struct VertexInput {
  @location(0) position: vec3<f32>,
}

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
  var output: VertexOutput;
  let worldPos = input.position * ball.scale + ball.translate;
  output.position = scene.viewProj * vec4<f32>(worldPos, 1.0);
  return output;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
  return vec4<f32>(ball.color, ball.opacity);
}
`;

const DEFAULT_RADIUS = 1;
const DEFAULT_SEGMENTS = 12;
const INSTANCE_UNIFORM_SIZE = 48;

const alphaBlendTarget = (format: GPUTextureFormat): GPUColorTargetState => ({
  format,
  blend: {
    color: {
      srcFactor: "src-alpha",
      dstFactor: "one-minus-src-alpha",
      operation: "add",
    },
    alpha: {
      srcFactor: "one",
      dstFactor: "one-minus-src-alpha",
      operation: "add",
    },
  },
});

export interface BallHelperOptions {
  /** Base mesh radius (default 1, alfrid `Geom.sphere(1, …)`). */
  radius?: number;
  /** Latitude/longitude subdivisions (default 12). */
  segments?: number;
  label?: string;
  /** From `createSceneBallPipelineLayout(device).pipelineLayout`. */
  pipelineLayout?: GPUPipelineLayout;
}

export interface BallDrawParams {
  position?: Vec3;
  /** Uniform scale or per-axis scale (default 1). */
  scale?: number | Vec3;
  color?: Vec3;
  opacity?: number;
}

function toVec3(value: number | Vec3): Vec3 {
  return typeof value === "number" ? [value, value, value] : value;
}

function writeInstanceUniform(buffer: Buffer, device: Device, params: BallDrawParams): void {
  const data = new Float32Array(12);
  const position = params.position ?? [0, 0, 0];
  const scale = toVec3(params.scale ?? 1);
  const color = params.color ?? [1, 1, 1];
  const opacity = params.opacity ?? 1;

  data[0] = position[0];
  data[1] = position[1];
  data[2] = position[2];
  data[4] = scale[0];
  data[5] = scale[1];
  data[6] = scale[2];
  data[8] = color[0];
  data[9] = color[1];
  data[10] = color[2];
  data[11] = opacity;

  buffer.write(device, data);
}

/**
 * Sphere mesh with per-draw position, scale, color, and opacity.
 * Group 0: scene `viewProj`; group 1: instance uniforms (alfrid `DrawBall` parity).
 */
export class BallHelper {
  readonly mesh: Mesh;
  private readonly device: Device;
  private readonly meshDraw: Draw;
  private readonly positionBuffer: Buffer;
  private readonly instanceBuffer: Buffer;
  private readonly instanceBindGroup: BindGroup;

  constructor(device: Device, options: BallHelperOptions = {}) {
    this.device = device;
    const radius = options.radius ?? DEFAULT_RADIUS;
    const segments = options.segments ?? DEFAULT_SEGMENTS;
    const label = options.label ?? "BallHelper";
    const { vertex, uniform } = BufferUsage;

    const positions = createSphereTriangleList(radius, segments);
    this.positionBuffer = Buffer.fromData(device, positions, vertex, `${label}-positions`);

    this.mesh = new Mesh(positions.length / 3).addVertexBuffer({
      buffer: this.positionBuffer,
      arrayStride: 12,
      attributes: [{ shaderLocation: 0, format: "float32x3", offset: 0 }],
      slot: 0,
    });

    const sceneBall = createSceneBallPipelineLayout(device, `${label}Scene`);
    const pipelineLayout = options.pipelineLayout ?? sceneBall.pipelineLayout;

    this.instanceBuffer = Buffer.create(
      device,
      Buffer.uniformSize(INSTANCE_UNIFORM_SIZE),
      uniform,
      `${label}-instance`,
    );

    this.instanceBindGroup = BindGroup.create(
      device,
      sceneBall.ballBindGroupLayout,
      this.instanceBuffer,
      0,
      `${label}-instance-bind-group`,
    );

    this.meshDraw = new Draw(device, BALL_SHADER, {
      label,
      layout: pipelineLayout,
      vertexBuffers: this.mesh.getVertexLayouts(),
      targets: [alphaBlendTarget(device.format)],
      depthStencil: {
        format: "depth24plus",
        // Transparent draws must not write depth (avoids sorting artifacts with opaque geometry).
        depthWriteEnabled: false,
        depthCompare: "less",
      },
      primitive: { topology: "triangle-list", cullMode: "back" },
    });
  }

  draw(
    passEncoder: GPURenderPassEncoder,
    sceneBindGroup: BindGroup,
    params: BallDrawParams = {},
  ): void {
    writeInstanceUniform(this.instanceBuffer, this.device, params);
    this.meshDraw.draw(passEncoder, this.mesh, [sceneBindGroup, this.instanceBindGroup]);
  }

  destroy(): void {
    this.positionBuffer.destroy();
    this.instanceBuffer.destroy();
  }
}
