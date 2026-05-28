import { mat4, vec3, type ReadonlyVec3 } from "gl-matrix";

const DEFAULT_UP = vec3.fromValues(0, 1, 0);

export class Camera {
  static readonly uniformFloatCount = 24;

  static uniformByteSize(): number {
    return Camera.uniformFloatCount * 4;
  }

  protected readonly viewMatrix: ReturnType<typeof mat4.create>;
  protected readonly projectionMatrix: ReturnType<typeof mat4.create>;
  private readonly viewProjectionMatrix: ReturnType<typeof mat4.create>;

  private readonly position = vec3.fromValues(0, 0, 1);
  private readonly target = vec3.fromValues(0, 0, 0);
  private readonly up = vec3.create();

  constructor() {
    this.viewMatrix = mat4.create();
    this.projectionMatrix = mat4.create();
    this.viewProjectionMatrix = mat4.create();
    mat4.identity(this.projectionMatrix);
    this.lookAt(this.position, this.target);
  }

  lookAt(eye: ReadonlyVec3, center: ReadonlyVec3, up: ReadonlyVec3 = DEFAULT_UP): this {
    vec3.set(this.position, eye[0], eye[1], eye[2]);
    vec3.set(this.target, center[0], center[1], center[2]);
    vec3.set(this.up, up[0], up[1], up[2]);
    mat4.lookAt(this.viewMatrix, this.position, this.target, this.up);
    return this;
  }

  getViewMatrix(): ReturnType<typeof mat4.create> {
    return this.viewMatrix;
  }

  getProjectionMatrix(): ReturnType<typeof mat4.create> {
    return this.projectionMatrix;
  }

  getViewProjectionMatrix(out?: Float32Array): Float32Array {
    const result = (out as unknown as ReturnType<typeof mat4.create>) ?? this.viewProjectionMatrix;
    mat4.multiply(result, this.projectionMatrix, this.viewMatrix);
    return result as unknown as Float32Array;
  }

  /**
   * Writes camera uniform data as:
   * - mat4 viewProj (16 floats)
   * - vec4 cameraRight (4 floats; w = 0)
   * - vec4 cameraUp (4 floats; w = 0)
   */
  writeUniformData(out: Float32Array, offset = 0): Float32Array {
    if (out.length < offset + Camera.uniformFloatCount) {
      throw new Error(
        `Camera uniform target is too small. Need at least ${offset + Camera.uniformFloatCount} floats.`,
      );
    }

    this.getViewProjectionMatrix(this.viewProjectionMatrix as unknown as Float32Array);
    out.set(this.viewProjectionMatrix, offset);

    // World-space camera basis from view-matrix rows (column-major storage).
    const view = this.viewMatrix;
    out[offset + 16] = view[0];
    out[offset + 17] = view[4];
    out[offset + 18] = view[8];
    out[offset + 19] = 0;
    out[offset + 20] = view[1];
    out[offset + 21] = view[5];
    out[offset + 22] = view[9];
    out[offset + 23] = 0;

    return out;
  }

  getPosition(): ReturnType<typeof vec3.clone> {
    return vec3.clone(this.position);
  }

  getLookAtTarget(): ReturnType<typeof vec3.clone> {
    return vec3.clone(this.target);
  }

  getFieldOfView(): number | undefined {
    return undefined;
  }

  protected updateProjection(): void {
    // Overridden by subclasses.
  }
}
