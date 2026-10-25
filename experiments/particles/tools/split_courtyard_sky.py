"""Split this courtyard's sky shell, preserving every vertex record byte-for-byte.

Scene-specific spatial selection in original PLY axes (+Y down, +Z forward).
Not a semantic sky classifier. Requires only Python's standard library.
"""
import argparse
import json
import math
import mmap
import os
import re
import struct
import tempfile
from pathlib import Path

from remove_courtyard_tree import read_layout, sha256


def sky_mask(x, y, z):
    height = -y
    roof_envelope = max(2.4, .36 * max(z, 0), .60 * abs(x))
    return height > roof_envelope or (x*x + y*y + z*z > 24*24 and height > 3)


def sky_edge_mask(x, y, z, rgb, log_scales):
    """Clean bright blue/neutral sky residue; protect the two pale central facades.

    This is a measured heuristic, not a semantic guarantee. Fine edge splats can
    contain both roof and sky, so minor fringes remain preferable to clipping roofs.
    """
    height = -y
    facade = (abs(x) < 1.5 and 15.5 < z < 19.5 and height < 5.0)
    facade |= (abs(x) < 1.25 and -4 < z < 1 and height < 1.8)
    if height <= .9 or facade:
        return False
    red, green, blue = rgb
    blue_sky = red > .42 and green > .47 and blue > .55 and blue >= red-.02 and blue >= green-.02
    neutral_haze = min(rgb) > .62 and max(log_scales) > math.log(.045)
    return blue_sky or neutral_haze


def separate_paths(paths):
    resolved = [Path(p).resolve() for p in paths]
    for i, a in enumerate(resolved):
        for b in resolved[:i]:
            if a == b or (a.exists() and b.exists() and a.samefile(b)):
                raise ValueError('Source, foreground, sky and report must all be separate files')
    return resolved


def split_sky(source, foreground, sky, overwrite=False):
    source, foreground, sky = separate_paths([source, foreground, sky])
    for output in (foreground, sky):
        if output.exists() and not overwrite:
            raise FileExistsError(f'{output} exists; use --overwrite to regenerate')
    temporary = []
    with source.open('rb') as file:
        header, count, stride, offsets = read_layout(file)
        if source.stat().st_size != len(header) + count * stride:
            raise ValueError('PLY payload size does not match its header')
        with mmap.mmap(file.fileno(), 0, access=mmap.ACCESS_READ) as data:
            mask = bytearray(count)
            unpack = struct.Struct('<f').unpack_from
            active_sky = active_foreground = 0
            properties = [line.split()[-1] for line in header.splitlines() if line.startswith(b'property ')]
            opacity_offset = properties.index(b'opacity') * 4 if b'opacity' in properties else None
            edge_names = [b'f_dc_0', b'f_dc_1', b'f_dc_2', b'scale_0', b'scale_1', b'scale_2']
            edge_offsets = [properties.index(name)*4 for name in edge_names] if all(name in properties for name in edge_names) else []
            for i in range(count):
                start = len(header) + i * stride
                point = [unpack(data, start + offset)[0] for offset in offsets]
                if not all(math.isfinite(v) for v in point):
                    raise ValueError(f'Non-finite position at vertex {i}')
                mask[i] = sky_mask(*point)
                if not mask[i] and edge_offsets:
                    values = [unpack(data, start + offset)[0] for offset in edge_offsets]
                    if not all(math.isfinite(v) for v in values):
                        raise ValueError(f'Non-finite color or scale at vertex {i}')
                    rgb = [max(0, min(1, .5 + .28209479*v)) for v in values[:3]]
                    mask[i] = sky_edge_mask(*point, rgb, values[3:])
                if opacity_offset is not None and unpack(data, start + opacity_offset)[0] >= math.log(1/254):
                    if mask[i]:
                        active_sky += 1
                    else:
                        active_foreground += 1
            sky_count = sum(mask)
            try:
                for output, selected, size in [(foreground, 0, count-sky_count), (sky, 1, sky_count)]:
                    output.parent.mkdir(parents=True, exist_ok=True)
                    with tempfile.NamedTemporaryFile(dir=output.parent, prefix=output.name+'.', suffix='.tmp', delete=False) as out:
                        temporary.append(Path(out.name))
                        out.write(re.sub(rb'(?m)^element vertex \d+', f'element vertex {size}'.encode(), header, count=1))
                        run = None
                        for i in range(count + 1):
                            include = i < count and mask[i] == selected
                            if include and run is None:
                                run = i
                            elif not include and run is not None:
                                out.write(data[len(header)+run*stride:len(header)+i*stride])
                                run = None
                # Both outputs are fully written before either destination is replaced.
                for temp, output in zip(temporary, [foreground, sky]):
                    os.replace(temp, output)
            finally:
                for temp in temporary:
                    temp.unlink(missing_ok=True)
    return {
        'source': source.name, 'foreground': foreground.name, 'sky': sky.name,
        'source_sha256': sha256(source), 'foreground_sha256': sha256(foreground), 'sky_sha256': sha256(sky),
        'source_records': count, 'foreground_records': count-sky_count, 'sky_records': sky_count,
        'active_foreground': active_foreground if opacity_offset is not None else None,
        'active_sky': active_sky if opacity_offset is not None else None,
        'coordinates': 'Original PLY: +Y down, +Z forward',
        'selection': 'h=-y; h > max(2.4, 0.36*max(z,0), 0.60*abs(x)) OR (length(x,y,z)>24 AND h>3)',
        'edge_selection': 'Above h=0.9, outside protected facades: RGB R>0.42,G>0.47,B>0.55,B>=R-0.02,B>=G-0.02 OR min(RGB)>0.62 and max(scale)>0.045. See sky_edge_mask for facade volumes.',
        'method': 'Scene-specific roof envelope plus color/scale edge cleanup. Both partitions preserve record bytes and order. No semantic labels, inpainting or relighting; some mixed roof/sky fringes may remain.',
    }


if __name__ == '__main__':
    project = Path(__file__).resolve().parent.parent
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--input', type=Path, default=project/'public/courtyard-no-tree.ply')
    parser.add_argument('--foreground', type=Path, default=project/'public/courtyard-foreground.ply')
    parser.add_argument('--sky', type=Path, default=project/'assets/courtyard-sky.ply')
    parser.add_argument('--report', type=Path, default=project/'assets/courtyard-sky-separation.json')
    parser.add_argument('--overwrite', action='store_true')
    args = parser.parse_args()
    separate_paths([args.input, args.foreground, args.sky, args.report])
    if args.report.exists() and not args.overwrite:
        parser.error('Report already exists; use --overwrite to regenerate')
    report = split_sky(args.input, args.foreground, args.sky, args.overwrite)
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(report, indent=2))
