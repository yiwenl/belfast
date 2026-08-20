# Camera Fit Utilities

Utility functions to easily calculate bounds for `OrthographicCamera` objects based on a target sphere or bounding box. Useful for tightly bounding a shadow camera to the actual content being illuminated.

## Import

```ts
import {
  fitOrthographicCameraToSphere,
  fitOrthographicCameraToBounds,
  type FitOrthographicCameraToSphereOptions,
  type FitOrthographicCameraToBoundsOptions,
} from "belfast";
```

## Methods

### `fitOrthographicCameraToSphere(options)`

Fits the orthographic camera frustum to a sphere bounds. Modifies and returns the `camera`.

| Field      | Type                       | Description                                         |
| ---------- | -------------------------- | --------------------------------------------------- |
| `camera`   | `OrthographicCamera`       | Camera to fit                                       |
| `center`   | `[number, number, number]` | Sphere center                                       |
| `radius`   | `number`                   | Sphere radius                                       |
| `eye`      | `[number, number, number]` | Position of camera                                  |
| `up?`      | `[number, number, number]` | Optional up vector                                  |
| `padding?` | `number`                   | Extra percentage to expand by (e.g., `0.5` is +50%) |

### `fitOrthographicCameraToBounds(options)`

Fits the orthographic camera frustum to an arbitrary list of bounding points. Modifies and returns the `camera`.

| Field      | Type                       | Description                   |
| ---------- | -------------------------- | ----------------------------- |
| `camera`   | `OrthographicCamera`       | Camera to fit                 |
| `points`   | `Vec3[]`                   | Points to encompass           |
| `eye`      | `[number, number, number]` | Position of camera            |
| `target`   | `[number, number, number]` | Look-at target                |
| `up?`      | `[number, number, number]` | Optional up vector            |
| `padding?` | `number`                   | Extra percentage to expand by |
