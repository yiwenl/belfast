export interface ScrubMedia extends EventTarget {
  currentTime: number;
  readonly duration: number;
  readonly readyState: number;
  readonly seeking: boolean;
  pause(): void;
}

/** Keep a paused video on the camera timeline, with at most one decoder seek in flight. */
export class VideoScrubber {
  private target = 0;
  private disposed = false;
  private readonly flush = () => {
    if (
      this.disposed ||
      this.video.readyState < 1 ||
      this.video.seeking ||
      !Number.isFinite(this.video.duration)
    )
      return;
    const target = Math.max(0, Math.min(this.target, this.video.duration));
    if (Math.abs(this.video.currentTime - target) > 0.00001) this.video.currentTime = target;
  };

  constructor(private readonly video: ScrubMedia) {
    video.pause();
    video.addEventListener("loadedmetadata", this.flush);
    video.addEventListener("durationchange", this.flush);
    video.addEventListener("seeked", this.flush);
  }

  setTime(seconds: number) {
    if (this.disposed || !Number.isFinite(seconds)) return;
    this.target = seconds;
    this.flush();
  }

  dispose() {
    this.disposed = true;
    this.video.removeEventListener("loadedmetadata", this.flush);
    this.video.removeEventListener("durationchange", this.flush);
    this.video.removeEventListener("seeked", this.flush);
    this.video.pause();
  }
}
