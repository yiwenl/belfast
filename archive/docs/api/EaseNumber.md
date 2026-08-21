# EaseNumber

Smoothly animates a numeric value toward a target using linear easing each enterframe.

## Import

```ts
import { EaseNumber } from "belfast";
```

Uses the [`scheduling`](https://www.npmjs.com/package/scheduling) package (`Scheduler.addEF`) — updates run automatically; no manual `update()` required.

## Constructor

```ts
new EaseNumber(value, (easing = 0.1));
```

| Argument | Description                 |
| -------- | --------------------------- |
| `value`  | Initial value and target    |
| `easing` | Lerp factor per frame (0–1) |

## Methods

| Method            | Description                                                            |
| ----------------- | ---------------------------------------------------------------------- |
| `setTo(n)`        | Set value and target immediately (applies `limit` clamping right away) |
| `add(delta)`      | Add to target                                                          |
| `limit(min, max)` | Clamp target                                                           |
| `destroy()`       | Remove enterframe callback (`Scheduler.removeEF`)                      |

## Properties

| Property      | Description            |
| ------------- | ---------------------- |
| `value`       | Current smoothed value |
| `targetValue` | Target (read-only)     |
| `easing`      | Lerp factor            |

Setting `value` updates the target (animation continues from current `value`).

## See also

- [OrbitalControl](OrbitalControl.md) — uses `EaseNumber` for radius and rotation
