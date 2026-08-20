# Utilities

Browser support checks and user-facing fallback when WebGPU is unavailable.

## Import

```ts
import { assertWebGPUSupport, showWebGPUUnavailableMessage } from "belfast";
```

## `assertWebGPUSupport(): Promise<void>`

Calls `Device.isSupported()`. If false:

1. Appends a full-screen message via `showWebGPUUnavailableMessage()`
2. Throws `Error("WebGPU is not supported.")`

Use at app startup before creating a canvas:

```ts
await assertWebGPUSupport();
const device = await Device.create(canvas);
```

## `showWebGPUUnavailableMessage(container?)`

Inserts a fixed overlay `div` with a short message suggesting a supported browser.

| Argument    | Default         | Description                 |
| ----------- | --------------- | --------------------------- |
| `container` | `document.body` | Where to append the message |

Does not throw. Safe to call directly if you only want UI feedback without an exception.

## See also

- [Device](Device.md) — `Device.isSupported()` for a non-throwing check
