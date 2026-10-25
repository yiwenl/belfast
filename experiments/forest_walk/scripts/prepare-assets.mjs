import { copyFile, mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import { resolve, join } from "node:path";
import { fileURLToPath } from "node:url";

const source = process.argv[2];
if (!source) throw new Error("Usage: pnpm prepare:assets -- <point-cloud output directory>");
const input = resolve(source === "--" ? process.argv[3] : source);
const names = await readdir(input);
const jsonNames = names.filter((name) => name.endsWith("_cameras.json"));
if (jsonNames.length !== 1)
  throw new Error("Expected exactly one *_cameras.json in the source directory");
const data = JSON.parse(await readFile(join(input, jsonNames[0]), "utf8"));
if (!Array.isArray(data.frames) || data.frames.length === 0)
  throw new Error("Camera JSON contains no frames");
const plyName = data.point_cloud;
if (
  typeof plyName !== "string" ||
  plyName.includes("/") ||
  plyName.includes("\\") ||
  !plyName.endsWith(".ply")
) {
  throw new Error("Camera JSON must name a PLY file in the same source directory");
}
const output = fileURLToPath(new URL("../public/", import.meta.url));
await mkdir(output, { recursive: true });
await copyFile(join(input, plyName), join(output, "forest.ply"));
const cameraFields = ({ camera_id, width, height, named_parameters }) => ({
  camera_id,
  width,
  height,
  named_parameters,
});
// Camera 152 onward has unreliable tracking; keep camera 151 as the inclusive endpoint.
const lastReliableCamera = 151;
const frames = data.frames
  .toSorted((a, b) => a.timestamp_seconds - b.timestamp_seconds)
  .slice(0, lastReliableCamera);
const cameras = {
  schema_version: data.schema_version,
  cameras: (data.cameras ?? []).map(cameraFields),
  undistorted_cameras: (data.undistorted_cameras ?? []).map(cameraFields),
  frames: frames.map(({ camera_id, timestamp_seconds, camera_to_world }) => ({
    camera_id,
    timestamp_seconds,
    camera_to_world,
  })),
};
await writeFile(join(output, "cameras.json"), JSON.stringify(cameras));
console.log(`Prepared forest.ply and ${cameras.frames.length} recorded cameras in ${output}`);
