import { mat4, vec3, type ReadonlyVec3 } from "gl-matrix";
import type { Camera } from "../camera/Camera";
import type { GeometryData } from "../helper/Geom";
import { Ray } from "../math/Ray";

export interface HitTestorOptions {
  /** Skip hit-testing on mousemove (only test on down / up). Default `false`. */
  skipMoveCheck?: boolean;
  /** Element to attach mouse/touch listeners to. Default `window`. */
  listenerTarget?: EventTarget;
  /**
   * Element whose CSS bounds define the hit-test viewport. Defaults to
   * `listenerTarget` when it is an element. Use this when the canvas drawing
   * buffer is scaled by devicePixelRatio.
   */
  viewportTarget?: Element;
}

export interface HitDetail {
  hit: vec3;
}

type HitEventType = "onHit" | "onDown" | "onUp";

interface MouseXY {
  x: number;
  y: number;
}

function getMouseXY(e: MouseEvent | TouchEvent): MouseXY {
  if ("touches" in e && e.touches.length > 0) {
    return { x: e.touches[0].clientX, y: e.touches[0].clientY };
  }
  const me = e as MouseEvent;
  return { x: me.clientX, y: me.clientY };
}

function hasViewportBounds(target: EventTarget | undefined): target is Element {
  return !!target && "getBoundingClientRect" in target;
}

function toResolutionXY(
  pointer: MouseXY,
  resolution: [number, number],
  viewportTarget?: Element,
): MouseXY {
  if (!viewportTarget) {
    return pointer;
  }

  const rect = viewportTarget.getBoundingClientRect();
  if (rect.width <= 0 || rect.height <= 0) {
    return pointer;
  }

  return {
    x: ((pointer.x - rect.left) / rect.width) * resolution[0],
    y: ((pointer.y - rect.top) / rect.height) * resolution[1],
  };
}

function dist2D(a: MouseXY, b: MouseXY): number {
  const dx = a.x - b.x;
  const dy = a.y - b.y;
  return Math.sqrt(dx * dx + dy * dy);
}

/**
 * CPU-side hit tester: casts a ray from the camera through the mouse position
 * and tests it against the triangles of a `GeometryData` object.
 *
 * Events (dispatched via `EventTarget`):
 * - `"onHit"`  — mousemove hit  → `event.detail.hit: vec3`
 * - `"onDown"` — mousedown hit  → `event.detail.hit: vec3`
 * - `"onUp"`   — mouseup (click confirmed within tolerance) or no hit
 */
export class HitTestor extends EventTarget {
  /** Pixel tolerance for distinguishing a click from a drag. */
  clickTolerance = 8;

  /** World-space model matrix applied to geometry before testing. */
  modelMatrix: ReturnType<typeof mat4.create>;

  /** Viewport resolution in pixels `[width, height]`. */
  resolution: [number, number];

  private readonly _camera: Camera;
  private readonly _faces: Float32Array[]; // each entry is 9 floats (3 vertices × 3 components)
  private readonly _ray: Ray;
  private readonly _skipMove: boolean;
  private readonly _listenerTarget: EventTarget;
  private readonly _viewportTarget?: Element;

  private _lastPos: MouseXY = { x: 0, y: 0 };
  private _firstPos: MouseXY = { x: 0, y: 0 };
  private _hit = vec3.fromValues(-999, -999, -999);

  private readonly _onDownBind: (e: Event) => void;
  private readonly _onMoveBind: (e: Event) => void;
  private readonly _onUpBind: () => void;

  constructor(
    geometry: GeometryData,
    camera: Camera,
    resolution?: [number, number],
    options: HitTestorOptions = {},
  ) {
    super();

    this._camera = camera;
    this.resolution = resolution ?? [window.innerWidth, window.innerHeight];
    this.modelMatrix = mat4.create();
    this._ray = new Ray([0, 0, 0], [0, 0, -1]);
    this._skipMove = options.skipMoveCheck ?? false;
    this._listenerTarget = options.listenerTarget ?? window;
    this._viewportTarget =
      options.viewportTarget ??
      (hasViewportBounds(this._listenerTarget) ? this._listenerTarget : undefined);

    // Pre-build face list from GeometryData
    this._faces = buildFaces(geometry);

    this._onDownBind = (e) => this._onDown(e as MouseEvent | TouchEvent);
    this._onMoveBind = (e) => this._onMove(e as MouseEvent | TouchEvent);
    this._onUpBind = () => this._onUp();

    this.connect();
  }

  connect(): void {
    this._listenerTarget.addEventListener("mousedown", this._onDownBind);
    this._listenerTarget.addEventListener("mousemove", this._onMoveBind);
    this._listenerTarget.addEventListener("mouseup", this._onUpBind);
    this._listenerTarget.addEventListener("touchstart", this._onDownBind);
    this._listenerTarget.addEventListener("touchmove", this._onMoveBind);
    this._listenerTarget.addEventListener("touchend", this._onUpBind);
  }

  disconnect(): void {
    this._listenerTarget.removeEventListener("mousedown", this._onDownBind);
    this._listenerTarget.removeEventListener("mousemove", this._onMoveBind);
    this._listenerTarget.removeEventListener("mouseup", this._onUpBind);
    this._listenerTarget.removeEventListener("touchstart", this._onDownBind);
    this._listenerTarget.removeEventListener("touchmove", this._onMoveBind);
    this._listenerTarget.removeEventListener("touchend", this._onUpBind);
  }

  /** The last successful hit position in world space. */
  get hit(): Readonly<vec3> {
    return this._hit;
  }

  // ── internals ──────────────────────────────────────────────

  private _checkHit(eventType: HitEventType = "onHit"): void {
    const camera = this._camera;
    if (!camera) {
      return;
    }

    // Convert CSS pointer coords to local viewport pixels, then NDC [-1, 1].
    const pointer = toResolutionXY(this._lastPos, this.resolution, this._viewportTarget);
    const mx = (pointer.x / this.resolution[0]) * 2.0 - 1.0;
    const my = -(pointer.y / this.resolution[1]) * 2.0 + 1.0;

    camera.generateRay([mx, my, 0], this._ray);

    let closestHit: vec3 | null = null;
    let closestDist = Infinity;

    const v0 = vec3.create();
    const v1 = vec3.create();
    const v2 = vec3.create();

    for (let i = 0; i < this._faces.length; i++) {
      const face = this._faces[i];

      // Transform vertices by model matrix
      vec3.transformMat4(v0, [face[0], face[1], face[2]] as ReadonlyVec3, this.modelMatrix);
      vec3.transformMat4(v1, [face[3], face[4], face[5]] as ReadonlyVec3, this.modelMatrix);
      vec3.transformMat4(v2, [face[6], face[7], face[8]] as ReadonlyVec3, this.modelMatrix);

      const t = this._ray.intersectTriangle(v0, v1, v2);
      if (t) {
        const d = vec3.dist(t, camera.getPosition());
        if (d < closestDist) {
          closestHit = vec3.clone(t);
          closestDist = d;
        }
      }
    }

    if (closestHit) {
      this._hit = vec3.clone(closestHit);
      this.dispatchEvent(new CustomEvent<HitDetail>(eventType, { detail: { hit: closestHit } }));
    } else {
      this.dispatchEvent(new CustomEvent("onUp"));
    }
  }

  private _onDown(e: MouseEvent | TouchEvent): void {
    this._firstPos = getMouseXY(e);
    this._lastPos = getMouseXY(e);
    this._checkHit("onDown");
  }

  private _onMove(e: MouseEvent | TouchEvent): void {
    this._lastPos = getMouseXY(e);
    if (!this._skipMove) {
      this._checkHit();
    }
  }

  private _onUp(): void {
    const d = dist2D(this._firstPos, this._lastPos);
    if (d < this.clickTolerance) {
      this._checkHit();
    }
  }
}

// ── helpers ────────────────────────────────────────────────

/**
 * Build a flat list of triangle vertex data from `GeometryData`.
 * Each entry is a Float32Array of 9 floats: [ax,ay,az, bx,by,bz, cx,cy,cz].
 */
function buildFaces(geometry: GeometryData): Float32Array[] {
  const { positions, indices } = geometry;
  const faces: Float32Array[] = [];

  for (let i = 0; i < indices.length; i += 3) {
    const ia = indices[i];
    const ib = indices[i + 1];
    const ic = indices[i + 2];
    faces.push(
      new Float32Array([
        positions[ia * 3],
        positions[ia * 3 + 1],
        positions[ia * 3 + 2],
        positions[ib * 3],
        positions[ib * 3 + 1],
        positions[ib * 3 + 2],
        positions[ic * 3],
        positions[ic * 3 + 1],
        positions[ic * 3 + 2],
      ]),
    );
  }

  return faces;
}
