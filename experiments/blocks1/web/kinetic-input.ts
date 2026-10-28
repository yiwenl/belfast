import type { ExperimentApp } from "../pkg/experiment";

type Point = { x: number; y: number };
type SkeletonFrame = {
  timestamp: number;
  space: string;
  palms: [Point | null, Point | null];
};

function isObject(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function readFrame(data: unknown): SkeletonFrame | null {
  if (typeof data !== "string") return null;
  let message: unknown;
  try {
    message = JSON.parse(data);
  } catch {
    return null;
  }
  if (
    !isObject(message) ||
    message.type !== "skeleton" ||
    !isFiniteNumber(message.timestamp) ||
    message.timestamp < 0 ||
    typeof message.mirror !== "boolean" ||
    !isObject(message.video) ||
    !isFiniteNumber(message.video.width) ||
    !isFiniteNumber(message.video.height) ||
    message.video.width <= 0 ||
    message.video.height <= 0 ||
    !Array.isArray(message.poses) ||
    message.poses.length > 1
  )
    return null;

  const { width, height } = message.video;
  const frame: SkeletonFrame = {
    timestamp: message.timestamp,
    space: `${width}:${height}:${message.mirror}`,
    palms: [null, null],
  };
  if (!message.poses.length) return frame;
  const pose: unknown = message.poses[0];
  if (!isObject(pose) || !Array.isArray(pose.keypoints) || pose.keypoints.length !== 33) {
    return null;
  }
  const points: unknown[] = pose.keypoints;
  frame.palms = (["left", "right"] as const).map((side) => {
    let x = 0;
    let y = 0;
    for (const name of ["wrist", "index", "pinky"]) {
      const point = points.find((value) => isObject(value) && value.name === `${side}_${name}`);
      if (
        !isObject(point) ||
        !isFiniteNumber(point.x) ||
        !isFiniteNumber(point.y) ||
        point.x < 0 ||
        point.x > width ||
        point.y < 0 ||
        point.y > height ||
        (point.score !== undefined && (!isFiniteNumber(point.score) || point.score < 0.5))
      )
        return null;
      x += point.x;
      y += point.y;
    }
    // Station already mirrors image X; do not flip it a second time.
    return { x: x / (3 * width), y: y / (3 * height) };
  }) as SkeletonFrame["palms"];
  return frame;
}

export function bindKineticInput(canvas: HTMLCanvasElement, app: ExperimentApp) {
  const status = { connection: "Disabled", hands: 0 };
  const listeners = new AbortController();
  const url = new URL("/kinetic-ws", location.href);
  url.protocol = location.protocol === "https:" ? "wss:" : "ws:";
  let enabled = false;
  let socket: WebSocket | undefined;
  let retry: number | undefined;
  let connectingSince = 0;
  let lastFrameAt = 0;
  let lastTimestamp: number | undefined;
  let space: string | undefined;
  const filtered: [Point | null, Point | null] = [null, null];

  function clear() {
    for (let hand = 0; hand < 2; hand++) {
      filtered[hand] = null;
      app.clearHand(hand);
    }
    status.hands = 0;
    lastFrameAt = 0;
  }

  function disconnect() {
    clearTimeout(retry);
    retry = undefined;
    const previous = socket;
    socket = undefined;
    previous?.close();
    clear();
    lastTimestamp = undefined;
    space = undefined;
  }

  function connect() {
    if (!enabled || listeners.signal.aborted || document.hidden || socket) return;
    status.connection = "Connecting…";
    connectingSince = performance.now();
    const current = new WebSocket(url);
    socket = current;
    current.addEventListener("open", () => {
      if (socket === current) status.connection = "Waiting for tracking";
    });
    current.addEventListener("message", (event: MessageEvent<unknown>) => {
      if (socket !== current) return;
      const frame = readFrame(event.data);
      if (!frame) {
        clear();
        status.connection = "Invalid tracking frame";
        return;
      }
      if (frame.timestamp === lastTimestamp) return;
      const now = performance.now();
      if (
        frame.space !== space ||
        (lastTimestamp !== undefined && frame.timestamp < lastTimestamp) ||
        now - lastFrameAt > 500
      )
        clear();
      const alpha = 1 - Math.exp(-Math.min(now - lastFrameAt, 100) / 60);
      lastFrameAt = now;
      lastTimestamp = frame.timestamp;
      space = frame.space;
      const rect = canvas.getBoundingClientRect();
      let hands = 0;
      frame.palms.forEach((point, hand) => {
        if (!point) {
          filtered[hand] = null;
          app.clearHand(hand);
          return;
        }
        hands++;
        let previous = filtered[hand];
        // Treat large tracking jumps as reacquisition, not a stroke across the scene.
        if (previous && Math.hypot(point.x - previous.x, point.y - previous.y) > 0.3) {
          app.clearHand(hand);
          previous = null;
        }
        const next = previous
          ? {
              x: previous.x + (point.x - previous.x) * alpha,
              y: previous.y + (point.y - previous.y) * alpha,
            }
          : point;
        filtered[hand] = next;
        app.handMove(hand, next.x * rect.width, next.y * rect.height, rect.width, rect.height);
      });
      status.hands = hands;
      status.connection = hands ? "Tracking" : "Connected · no hands";
    });
    current.addEventListener("error", () => {
      if (socket === current) current.close();
    });
    current.addEventListener("close", () => {
      if (socket !== current) return;
      disconnect();
      status.connection = "Disconnected · retrying";
      retry = window.setTimeout(connect, 2000);
    });
  }

  // A stale stream must not bridge an old hand position to a newly tracked one.
  const watchdog = window.setInterval(() => {
    if (!socket) return;
    const now = performance.now();
    if (socket.readyState === WebSocket.CONNECTING && now - connectingSince > 8000) socket.close();
    if (lastFrameAt && now - lastFrameAt > 500) {
      clear();
      status.connection = "Waiting for tracking";
    }
  }, 250);
  window.addEventListener("resize", clear, { signal: listeners.signal });
  document.addEventListener(
    "visibilitychange",
    () => {
      disconnect();
      if (enabled) {
        status.connection = document.hidden ? "Hidden · suspended" : "Connecting…";
        connect();
      }
    },
    { signal: listeners.signal },
  );

  return {
    status,
    setEnabled(value: boolean) {
      if (enabled === value) return;
      enabled = value;
      disconnect();
      status.connection = enabled ? "Connecting…" : "Disabled";
      connect();
    },
    dispose() {
      enabled = false;
      listeners.abort();
      clearInterval(watchdog);
      disconnect();
    },
  };
}
