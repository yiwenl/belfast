import assert from "node:assert/strict";
import test from "node:test";
import { VideoScrubber } from "../web/video-sync.ts";

// Models the asynchronous media boundary; the scheduler itself is the real implementation.
class Media extends EventTarget {
  readyState = 0;
  duration = Number.NaN;
  seeking = false;
  paused = false;
  seeks: number[] = [];
  private time = 0;
  get currentTime() {
    return this.time;
  }
  set currentTime(time: number) {
    this.time = time;
    this.seeks.push(time);
    this.seeking = true;
  }
  pause() {
    this.paused = true;
  }
  ready() {
    this.readyState = 2;
    this.duration = 63.252;
    this.dispatchEvent(new Event("loadedmetadata"));
  }
  finish() {
    this.seeking = false;
    this.dispatchEvent(new Event("seeked"));
  }
}

test("waits for metadata, then seeks to the latest requested camera time", () => {
  const media = new Media();
  const sync = new VideoScrubber(media);
  sync.setTime(0.3);
  sync.setTime(3.75);
  assert.deepEqual(media.seeks, []);
  media.ready();
  assert.deepEqual(media.seeks, [3.75]);
  assert.equal(media.paused, true);
  sync.dispose();
});

test("coalesces pending seeks, supports reversal and settles without repeated decoding", () => {
  const media = new Media();
  media.ready();
  const sync = new VideoScrubber(media);
  sync.setTime(1.5);
  sync.setTime(5.5);
  sync.setTime(2.25);
  assert.deepEqual(media.seeks, [1.5]);
  media.finish();
  assert.deepEqual(media.seeks, [1.5, 2.25]);
  media.finish();
  sync.setTime(2.25);
  assert.deepEqual(media.seeks, [1.5, 2.25]);
  sync.setTime(0);
  media.finish();
  assert.equal(media.currentTime, 0);
  sync.dispose();
});

test("clamps to media bounds, ignores invalid times and stops pending work on disposal", () => {
  const media = new Media();
  media.ready();
  const sync = new VideoScrubber(media);
  sync.setTime(100);
  assert.equal(media.currentTime, 63.252);
  media.finish();
  sync.setTime(Number.NaN);
  sync.setTime(Number.POSITIVE_INFINITY);
  assert.equal(media.seeks.length, 1);
  sync.setTime(-2);
  assert.equal(media.currentTime, 0);
  sync.setTime(10);
  sync.dispose();
  media.finish();
  sync.setTime(20);
  assert.deepEqual(media.seeks, [63.252, 0]);
});
