import type { ExperimentApp } from "../pkg/experiment";

function imageTracking(message: unknown): {
  nose: [number, number] | null;
  handRaised: boolean;
} | null {
  if (!message || typeof message !== "object") return null;
  const frame = message as {
    type?: unknown;
    video?: { width?: unknown; height?: unknown };
    poses?: { keypoints?: unknown }[];
  };
  const width = frame.video?.width;
  const height = frame.video?.height;
  if (
    frame.type !== "skeleton" ||
    typeof width !== "number" ||
    !Number.isFinite(width) ||
    width <= 0 ||
    typeof height !== "number" ||
    !Number.isFinite(height) ||
    height <= 0 ||
    !Array.isArray(frame.poses)
  )
    return null;
  const points = frame.poses[0]?.keypoints;
  if (!Array.isArray(points)) return null;
  const point = (name: string): { x: number; y: number } | null => {
    const value = points.find((p) => p && p.name === name);
    if (
      !value ||
      typeof value.x !== "number" ||
      !Number.isFinite(value.x) ||
      typeof value.y !== "number" ||
      !Number.isFinite(value.y) ||
      (value.score !== undefined &&
        (typeof value.score !== "number" ||
          !Number.isFinite(value.score) ||
          value.score < 0.5 ||
          value.score > 1))
    )
      return null;
    return value;
  };
  const nose = point("nose");
  const leftWrist = point("left_wrist");
  const leftShoulder = point("left_shoulder");
  const rightWrist = point("right_wrist");
  const rightShoulder = point("right_shoulder");
  // Station has already mirrored X. Convert image Y-down to scene Y-up.
  const clamp = (value: number) => Math.max(-1, Math.min(1, value));
  return {
    nose: nose ? [clamp((2 * nose.x) / width - 1), clamp(1 - (2 * nose.y) / height)] : null,
    // Image Y increases downward; either wrist above its own shoulder holds the effect.
    handRaised: Boolean(
      (leftWrist && leftShoulder && leftWrist.y < leftShoulder.y) ||
      (rightWrist && rightShoulder && rightWrist.y < rightShoulder.y),
    ),
  };
}

export function bindInteractionControls(
  app: ExperimentApp,
  onModeChange: (active: boolean) => void,
) {
  const button = document.querySelector<HTMLButtonElement>("#interaction-toggle")!;
  const scale = document.querySelector<HTMLInputElement>("#interaction-scale")!;
  const scaleValue = document.querySelector<HTMLOutputElement>("#interaction-scale-value")!;
  const status = document.querySelector<HTMLElement>("#interaction-status")!;
  const listeners = new AbortController();
  let active = false;
  let ready = false;
  let locked = false;
  let socket: WebSocket | undefined;
  let retry: ReturnType<typeof setTimeout> | undefined;
  let monitor: ReturnType<typeof setInterval> | undefined;
  let lastMessage = 0;
  let lastTracking = 0;
  let lastNose = 0;
  let nose: [number, number] | null = null;
  let dragging = false;

  function setDragging(value: boolean) {
    if (dragging === value) return;
    dragging = value;
    app.setInteractionDragging(value);
  }

  function resetTracking() {
    nose = null;
    setDragging(false);
  }

  function showStatus(text: string) {
    if (status.textContent !== text) status.textContent = text;
  }

  function updateControls() {
    button.disabled = !ready || locked;
    scale.disabled = !ready || locked;
    button.textContent = active ? "退出互動模式" : "進入互動模式";
    button.setAttribute("aria-pressed", String(active));
  }

  function applyScale() {
    const amount = scale.valueAsNumber;
    if (!Number.isFinite(amount) || amount < 0 || amount > 5) return;
    scaleValue.value = `${amount.toFixed(2)}×`;
    if (active && nose && performance.now() - lastNose <= 250) {
      app.setNosePosition(nose[0] * amount, nose[1] * amount);
    }
  }

  function connect() {
    if (!active) return;
    showStatus("連線到 Station…");
    const url = new URL("./kinetic", window.location.href);
    url.protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    url.search = "";
    url.hash = "";
    const current = new WebSocket(url);
    socket = current;
    lastMessage = performance.now();
    current.addEventListener("open", () => {
      if (socket !== current) return;
      lastMessage = performance.now();
      showStatus("已連線，等待人物追蹤…");
    });
    current.addEventListener("message", (event) => {
      if (!active || socket !== current) return;
      lastMessage = performance.now();
      let tracking: ReturnType<typeof imageTracking> = null;
      try {
        tracking = imageTracking(JSON.parse(event.data));
      } catch {
        tracking = null;
      }
      if (!tracking) {
        resetTracking();
        showStatus("未偵測到人物，保持視角");
        return;
      }
      lastTracking = lastMessage;
      nose = tracking.nose;
      if (nose) {
        lastNose = lastMessage;
        applyScale();
      }
      setDragging(tracking.handRaised);
      showStatus(
        `${nose ? "鼻子追蹤中" : "未偵測到鼻子，保持視角"} · ${dragging ? "舉手拖曳中" : "未舉手"}`,
      );
    });
    current.addEventListener("error", () => {
      if (socket === current) showStatus("Station 連線失敗，正在重連…");
    });
    current.addEventListener("close", () => {
      if (!active || socket !== current) return;
      socket = undefined;
      resetTracking();
      showStatus("Station 已斷線，正在重連…");
      retry = setTimeout(connect, 1500);
    });
  }

  function stop() {
    if (!active) return;
    active = false;
    clearTimeout(retry);
    clearInterval(monitor);
    const previous = socket;
    socket = undefined;
    previous?.close();
    resetTracking();
    app.setInteractionMode(false);
    onModeChange(false);
    showStatus("尚未進入互動模式");
    updateControls();
  }

  button.addEventListener(
    "click",
    () => {
      if (!ready || locked) return;
      if (active) {
        stop();
        return;
      }
      active = true;
      onModeChange(true);
      app.setInteractionMode(true);
      updateControls();
      connect();
      monitor = setInterval(() => {
        const now = performance.now();
        if (socket?.readyState === WebSocket.OPEN && now - lastTracking > 250) {
          resetTracking();
          showStatus("等待人物追蹤，保持視角");
        }
        if (socket && now - lastMessage > 5000) socket.close();
      }, 250);
    },
    { signal: listeners.signal },
  );
  scale.addEventListener("input", applyScale, { signal: listeners.signal });
  applyScale();
  updateControls();

  return {
    setReady() {
      ready = true;
      updateControls();
    },
    setLocked(value: boolean) {
      locked = value;
      if (locked) stop();
      updateControls();
    },
    dispose() {
      stop();
      listeners.abort();
      ready = false;
      updateControls();
    },
  };
}
