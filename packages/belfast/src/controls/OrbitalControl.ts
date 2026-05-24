import Scheduler from "scheduling";
import type { Camera } from "../camera/Camera";
import type { MutVec3, Vec3 } from "../math/types";
import { EaseNumber } from "../utils/EaseNumber";

export interface OrbitalControlOptions {
  listenerTarget?: HTMLElement;
  center?: Vec3;
  radius?: number;
  up?: Vec3;
  sensitivity?: number;
  /** Multiplier for wheel zoom (default 1). */
  zoomSpeed?: number;
  /** Multiplier for middle-mouse / Shift+drag pan (default 0.01). */
  panSpeed?: number;
}

interface MousePoint {
  x: number;
  y: number;
}

function getMouse(event: MouseEvent | TouchEvent, target: MousePoint): MousePoint {
  if ("touches" in event && event.touches.length > 0) {
    target.x = event.touches[0].pageX;
    target.y = event.touches[0].pageY;
  } else if ("clientX" in event) {
    target.x = event.clientX;
    target.y = event.clientY;
  }
  return target;
}

/** Normalized scroll amount; handles pixel, line, and page deltaMode. */
function normalizeWheelDelta(event: WheelEvent): number {
  let delta = event.deltaY;
  switch (event.deltaMode) {
    case WheelEvent.DOM_DELTA_LINE:
      delta *= 16;
      break;
    case WheelEvent.DOM_DELTA_PAGE:
      delta *= 100;
      break;
  }
  return -delta / 120;
}

function normalizeVec3(v: Vec3): [number, number, number] {
  const len = Math.hypot(v[0], v[1], v[2]);
  if (len === 0) {
    return [0, 0, 0];
  }
  return [v[0] / len, v[1] / len, v[2] / len];
}

function cross(a: Vec3, b: Vec3): [number, number, number] {
  return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
}

export class OrbitalControl {
  readonly radius: EaseNumber;
  readonly position: MutVec3 = [0, 0, 0];
  positionOffset: MutVec3 = [0, 0, 0];
  center: MutVec3;
  sensitivity = 1;
  zoomSpeed = 1;
  panSpeed = 0.01;

  private readonly _camera: Camera;
  private readonly _listenerTarget: HTMLElement;
  private readonly _up: Vec3;
  private readonly _rx: EaseNumber;
  private readonly _ry: EaseNumber;
  private readonly _mouse: MousePoint = { x: 0, y: 0 };
  private readonly _preMouse: MousePoint = { x: 0, y: 0 };
  private readonly _panCenterStart: MutVec3 = [0, 0, 0];
  private readonly _efIndex: number;

  private _preRX = 0;
  private _preRY = 0;
  private _isLockZoom = false;
  private _isLockRotation = false;
  private _isLockPan = false;
  private _isInvert = false;
  private _isMouseDown = false;
  private _isPanning = false;
  private _destroyed = false;

  private readonly _wheelBind: (e: WheelEvent) => void;
  private readonly _downBind: (e: MouseEvent | TouchEvent) => void;
  private readonly _moveBind: (e: MouseEvent | TouchEvent) => void;
  private readonly _upBind: () => void;

  constructor(camera: Camera, options: OrbitalControlOptions = {}) {
    this._camera = camera;
    this._listenerTarget = options.listenerTarget ?? document.body;
    this.center = options.center ? [...options.center] : [0, 0, 0];
    this._up = options.up ? [...options.up] : [0, 1, 0];
    this.sensitivity = options.sensitivity ?? 1;
    this.zoomSpeed = options.zoomSpeed ?? 1;
    this.panSpeed = options.panSpeed ?? 0.01;

    const initialRadius = options.radius ?? 10;
    this.radius = new EaseNumber(initialRadius);
    this.position[2] = this.radius.value;

    this._rx = new EaseNumber(0);
    this._rx.limit(-Math.PI / 2, Math.PI / 2);
    this._ry = new EaseNumber(0);

    this._wheelBind = (e) => this._onWheel(e);
    this._downBind = (e) => this._onDown(e);
    this._moveBind = (e) => this._onMove(e);
    this._upBind = () => this._onUp();

    this.connect();
    this._efIndex = Scheduler.addEF(() => this._loop());
  }

  connect(): void {
    this.disconnect();
    this._listenerTarget.addEventListener("wheel", this._wheelBind, { passive: false });
    this._listenerTarget.addEventListener("mousedown", this._downBind);
    this._listenerTarget.addEventListener("touchstart", this._downBind, { passive: false });
    this._listenerTarget.addEventListener("mousemove", this._moveBind);
    this._listenerTarget.addEventListener("touchmove", this._moveBind, { passive: false });
    window.addEventListener("touchend", this._upBind);
    window.addEventListener("mouseup", this._upBind);
  }

  disconnect(): void {
    this._listenerTarget.removeEventListener("wheel", this._wheelBind);
    this._listenerTarget.removeEventListener("mousedown", this._downBind);
    this._listenerTarget.removeEventListener("touchstart", this._downBind);
    this._listenerTarget.removeEventListener("mousemove", this._moveBind);
    this._listenerTarget.removeEventListener("touchmove", this._moveBind);
    window.removeEventListener("touchend", this._upBind);
    window.removeEventListener("mouseup", this._upBind);
  }

  destroy(): void {
    if (this._destroyed) {
      return;
    }
    this._destroyed = true;
    this.disconnect();
    Scheduler.removeEF(this._efIndex);
    this.radius.destroy();
    this._rx.destroy();
    this._ry.destroy();
  }

  lock(value = true): void {
    this._isLockZoom = value;
    this._isLockRotation = value;
    this._isLockPan = value;
    this._isMouseDown = false;
    this._isPanning = false;
  }

  lockZoom(value = true): void {
    this._isLockZoom = value;
  }

  lockRotation(value = true): void {
    this._isLockRotation = value;
  }

  lockPan(value = true): void {
    this._isLockPan = value;
  }

  inverseControl(isInvert = true): void {
    this._isInvert = isInvert;
  }

  update(): void {
    this._updatePosition();
  }

  get rx(): EaseNumber {
    return this._rx;
  }

  get ry(): EaseNumber {
    return this._ry;
  }

  private _loop(): void {
    if (this._destroyed) {
      return;
    }
    this._updatePosition();
    this._updateCamera();
  }

  private _updatePosition(): void {
    const rx = this._rx.value;
    const ry = this._ry.value;
    const r = this.radius.value;

    this.position[1] = Math.sin(rx) * r;
    const tr = Math.cos(rx) * r;
    this.position[0] = Math.cos(ry + Math.PI * 0.5) * tr;
    this.position[2] = Math.sin(ry + Math.PI * 0.5) * tr;

    this.position[0] += this.positionOffset[0];
    this.position[1] += this.positionOffset[1];
    this.position[2] += this.positionOffset[2];
  }

  private _updateCamera(): void {
    this._camera.lookAt(this.position, this.center, this._up);
  }

  private _isPanInput(event: MouseEvent | TouchEvent): boolean {
    if (!("button" in event)) {
      return false;
    }
    return event.button === 1 || (event.button === 0 && event.shiftKey);
  }

  private _panByPixels(diffX: number, diffY: number): void {
    this._updatePosition();
    const eye: Vec3 = [this.position[0], this.position[1], this.position[2]];
    const forward = normalizeVec3([
      this.center[0] - eye[0],
      this.center[1] - eye[1],
      this.center[2] - eye[2],
    ]);
    const right = normalizeVec3(cross(forward, this._up));
    const camUp = normalizeVec3(cross(right, forward));

    const scale = this.panSpeed * this.sensitivity;
    this.center[0] = this._panCenterStart[0] - right[0] * diffX * scale + camUp[0] * diffY * scale;
    this.center[1] = this._panCenterStart[1] - right[1] * diffX * scale + camUp[1] * diffY * scale;
    this.center[2] = this._panCenterStart[2] - right[2] * diffX * scale + camUp[2] * diffY * scale;
  }

  private _onDown(event: MouseEvent | TouchEvent): void {
    getMouse(event, this._mouse);
    getMouse(event, this._preMouse);

    if (this._isPanInput(event) && !this._isLockPan) {
      this._isPanning = true;
      this._isMouseDown = false;
      this._panCenterStart[0] = this.center[0];
      this._panCenterStart[1] = this.center[1];
      this._panCenterStart[2] = this.center[2];
      return;
    }

    if (this._isLockRotation) {
      return;
    }
    this._isPanning = false;
    this._isMouseDown = true;
    this._preRX = this._rx.targetValue;
    this._preRY = this._ry.targetValue;
  }

  private _onMove(event: MouseEvent | TouchEvent): void {
    getMouse(event, this._mouse);
    if ("touches" in event) {
      event.preventDefault();
    }

    if (this._isPanning) {
      if (this._isLockPan) {
        return;
      }
      const diffX = this._mouse.x - this._preMouse.x;
      const diffY = this._mouse.y - this._preMouse.y;
      this._panByPixels(diffX, diffY);
      return;
    }

    if (this._isLockRotation || !this._isMouseDown) {
      return;
    }

    let diffX = -(this._mouse.x - this._preMouse.x);
    if (this._isInvert) {
      diffX *= -1;
    }
    this._ry.value = this._preRY - diffX * 0.01 * this.sensitivity;

    let diffY = -(this._mouse.y - this._preMouse.y);
    if (this._isInvert) {
      diffY *= -1;
    }
    this._rx.value = this._preRX - diffY * 0.01 * this.sensitivity;
  }

  private _onUp(): void {
    this._isMouseDown = false;
    this._isPanning = false;
  }

  private _onWheel(event: WheelEvent): void {
    if (this._isLockZoom) {
      return;
    }
    event.preventDefault();
    const value = normalizeWheelDelta(event) * this.zoomSpeed;
    this.radius.add(-value * 2);
    if (this.radius.targetValue < 0) {
      this.radius.setTo(0.0001);
    }
  }
}
