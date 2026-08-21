import Scheduler from "scheduling";

export class EaseNumber {
  easing: number;
  private _value: number;
  private _targetValue: number;
  private _min?: number;
  private _max?: number;
  private readonly _efIndex: number;

  constructor(value: number, easing = 0.1) {
    this.easing = easing;
    this._value = value;
    this._targetValue = value;
    this._efIndex = Scheduler.addEF(() => this._update());
  }

  private _update(): void {
    const minDiff = 0.0001;
    this._checkLimit();
    this._value += (this._targetValue - this._value) * this.easing;
    if (Math.abs(this._targetValue - this._value) < minDiff) {
      this._value = this._targetValue;
    }
  }

  setTo(value: number): void {
    this._targetValue = this._value = value;
    this._checkLimit();
    this._value = this._targetValue;
  }

  add(delta: number): void {
    this._targetValue += delta;
    this._checkLimit();
  }

  limit(min: number, max: number): void {
    if (min > max) {
      this.limit(max, min);
      return;
    }
    this._min = min;
    this._max = max;
    this._checkLimit();
  }

  private _checkLimit(): void {
    if (this._min !== undefined && this._targetValue < this._min) {
      this._targetValue = this._min;
    }
    if (this._max !== undefined && this._targetValue > this._max) {
      this._targetValue = this._max;
    }
  }

  destroy(): void {
    Scheduler.removeEF(this._efIndex);
  }

  set value(value: number) {
    this._targetValue = value;
  }

  get value(): number {
    return this._value;
  }

  get targetValue(): number {
    return this._targetValue;
  }
}
