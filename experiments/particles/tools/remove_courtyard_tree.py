"""Remove the measured central tree from courtyard.ply without rewriting kept attributes.

Python 3 standard library only. This mask is specific to this courtyard scan,
in original PLY coordinates (+Y down, +Z forward), not a semantic tree detector.
"""
import argparse
import hashlib
import json
import math
import mmap
import os
import re
import struct
import tempfile
from pathlib import Path


# Measured from front/side/top projections of the active Gaussian centers.
CROWN = {"x": (-1.65, 1.70), "y": (-2.60, 0.72), "z": (5.10, 6.80)}
ROOT = {"x": 0.05, "z": 5.98, "radius_x": 0.23, "radius_z": 0.24, "y": (0.72, 0.80)}


def tree_mask(x, y, z):
    if (CROWN["x"][0] <= x <= CROWN["x"][1]
            and CROWN["y"][0] <= y <= CROWN["y"][1]
            and CROWN["z"][0] <= z <= CROWN["z"][1]):
        return True
    if ROOT["y"][0] < y < ROOT["y"][1]:
        return ((x - ROOT["x"]) / ROOT["radius_x"]) ** 2 + ((z - ROOT["z"]) / ROOT["radius_z"]) ** 2 <= 1
    return False


def read_layout(file):
    header = bytearray()
    while True:
        line = file.readline(65537)
        header.extend(line)
        if len(header) > 65536 or not line:
            raise ValueError("Missing or oversized PLY header")
        if line.strip() == b"end_header":
            break
    lines = header.decode("ascii").splitlines()
    if lines[:2] != ["ply", "format binary_little_endian 1.0"]:
        raise ValueError("Expected binary little-endian PLY")
    count, properties = None, []
    for line in lines[2:]:
        parts = line.split()
        if parts[:1] == ["element"]:
            if len(parts) != 3 or parts[1] != "vertex" or count is not None:
                raise ValueError("This asset tool expects one vertex element only")
            count = int(parts[2])
        elif parts[:1] == ["property"]:
            if count is None or len(parts) != 3 or parts[1] not in ("float", "float32"):
                raise ValueError("This asset tool expects scalar float vertex properties")
            if parts[2] in properties:
                raise ValueError("Duplicate vertex property")
            properties.append(parts[2])
    if count is None or count < 1 or not all(name in properties for name in ("x", "y", "z")):
        raise ValueError("Missing vertices or position attributes")
    return bytes(header), count, len(properties) * 4, [properties.index(name) * 4 for name in ("x", "y", "z")]


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as file:
        for chunk in iter(lambda: file.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def remove_tree(source, output, overwrite=False):
    source, output = Path(source).resolve(), Path(output).resolve()
    if source == output or (output.exists() and source.samefile(output)):
        raise ValueError("Output must be a separate file; the original is always preserved")
    if output.exists() and not overwrite:
        raise FileExistsError(f"{output} already exists; use --overwrite to regenerate it")
    removed = 0
    removed_min, removed_max = [math.inf] * 3, [-math.inf] * 3
    with source.open("rb") as file:
        header, count, stride, offsets = read_layout(file)
        expected = len(header) + count * stride
        if source.stat().st_size != expected:
            raise ValueError("PLY payload size does not match its header (truncated or unsupported trailing data)")
        with mmap.mmap(file.fileno(), 0, access=mmap.ACCESS_READ) as data:
            keep = bytearray(b"\x01") * count
            unpack = struct.Struct("<f").unpack_from
            for i in range(count):
                start = len(header) + i * stride
                x, y, z = (unpack(data, start + offset)[0] for offset in offsets)
                if not all(math.isfinite(v) for v in (x, y, z)):
                    raise ValueError(f"Non-finite position at vertex {i}")
                if tree_mask(x, y, z):
                    keep[i] = 0
                    removed += 1
                    for axis, value in enumerate((x, y, z)):
                        removed_min[axis] = min(removed_min[axis], value)
                        removed_max[axis] = max(removed_max[axis], value)
            updated_header = re.sub(rb"(?m)^element vertex \d+", f"element vertex {count - removed}".encode(), header, count=1)
            output.parent.mkdir(parents=True, exist_ok=True)
            temporary = None
            try:
                with tempfile.NamedTemporaryFile(dir=output.parent, prefix=output.name + ".", suffix=".tmp", delete=False) as destination:
                    temporary = Path(destination.name)
                    destination.write(updated_header)
                    # Write contiguous retained runs verbatim, including opacity/scale/rotation.
                    run_start = 0
                    for i, retain in enumerate(keep):
                        if not retain:
                            if i > run_start:
                                destination.write(data[len(header)+run_start*stride:len(header)+i*stride])
                            run_start = i + 1
                    if run_start < count:
                        destination.write(data[len(header)+run_start*stride:])
                os.replace(temporary, output)
                temporary = None
            finally:
                if temporary is not None:
                    temporary.unlink(missing_ok=True)
    return {
        "source": source.name,
        "output": output.name,
        "source_sha256": sha256(source),
        "output_sha256": sha256(output),
        "source_records": count,
        "removed_records": removed,
        "retained_records": count - removed,
        "removed_position_min": removed_min if removed else None,
        "removed_position_max": removed_max if removed else None,
        "coordinates": "Original PLY: +Y down, +Z forward",
        "crown_volume": CROWN,
        "root_volume": ROOT,
        "method": "Spatial Gaussian-center selection; retained records copied byte-for-byte. No background inpainting.",
    }


if __name__ == "__main__":
    project = Path(__file__).resolve().parent.parent
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, default=project / "public/courtyard.ply")
    parser.add_argument("--output", type=Path, default=project / "public/courtyard-no-tree.ply")
    parser.add_argument("--report", type=Path, default=project / "assets/courtyard-tree-removal.json")
    parser.add_argument("--overwrite", action="store_true")
    args = parser.parse_args()
    for asset in (args.input, args.output):
        if (args.report.resolve() == asset.resolve()
                or (args.report.exists() and asset.exists() and args.report.samefile(asset))):
            parser.error("The report must be separate from both PLY files")
    report = remove_tree(args.input, args.output, args.overwrite)
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))
