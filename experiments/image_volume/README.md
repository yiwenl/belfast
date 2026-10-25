# Image volume

190 photographs stacked into one `texture_3d`, from z = -1 to z = 1. A 1×1 plane samples that volume with its rotated world position.

## Run

From the repository root, with Rust's `wasm32-unknown-unknown` target, `wasm-pack`, Node and pnpm installed:

```sh
pnpm install
pnpm --dir experiments/image_volume dev
```

Open <http://localhost:5180>. WebGPU requires a supported browser on localhost or HTTPS.

- Drag: rotate the plane.
- Scroll: zoom the camera. The camera stays in front.
- 正面顯示切面: on, the plane stays a square and only the sample coordinates rotate. Off, the plane tilts in the cube.
- 顯示全部切面: draws every photograph as a translucent slab, back to front, and turns 正面顯示切面 off. The camera moves up and to the side so the stack is visible, and returns to the front when the toggle goes off.

```sh
pnpm --dir experiments/image_volume build
pnpm --dir experiments/image_volume test
cargo test -p belfast-experiment-image-volume
```

## Frames

The raw sequence in `experiments/img_sequence/` is gitignored (190 JPEGs, 6864×5152, 2.4 GB). `public/frames/000.jpg` … `189.jpg` are 1024×1024 center crops. `_DSF6741` is frame 0 (z = -1). Regenerate them with `sips`:

```sh
mkdir -p experiments/image_volume/public/frames
i=0; for f in experiments/img_sequence/*.JPG; do
  sips -c 5152 5152 "$f" --out /tmp/sq.jpg >/dev/null
  sips -z 1024 1024 -s formatOptions 85 /tmp/sq.jpg --out "$(printf experiments/image_volume/public/frames/%03d.jpg $i)" >/dev/null
  i=$((i+1)); done
```

The volume is about 800 MB of RGBA8 on the GPU. One `texture_3d` binding holds every frame.
