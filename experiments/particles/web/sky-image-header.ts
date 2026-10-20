/** Read dimensions without decoding pixels. Only the first 256 KiB is needed. */
export function panoramaDimensions(bytes: Uint8Array): [number, number] {
  const v = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const tag = (at: number, s: string) => [...s].every((c, i) => bytes[at + i] === c.charCodeAt(0));
  if (
    bytes.length >= 24 &&
    [137, 80, 78, 71, 13, 10, 26, 10].every((b, i) => bytes[i] === b) &&
    tag(12, "IHDR")
  ) {
    return [v.getUint32(16), v.getUint32(20)];
  }
  if (bytes[0] === 255 && bytes[1] === 216) {
    let offset = 2;
    while (offset + 4 <= bytes.length && bytes[offset] === 255) {
      while (bytes[offset] === 255) offset++;
      const marker = bytes[offset++];
      if (marker === 218 || marker === 217) break;
      if (marker === 1 || (marker >= 208 && marker <= 215)) continue;
      if (offset + 2 > bytes.length) break;
      const size = v.getUint16(offset);
      if (size < 2 || offset + size > bytes.length) break;
      if (
        [192, 193, 194, 195, 197, 198, 199, 201, 202, 203, 205, 206, 207].includes(marker) &&
        size >= 8
      ) {
        return [v.getUint16(offset + 5), v.getUint16(offset + 3)];
      }
      offset += size;
    }
  }
  if (bytes.length >= 20 && tag(0, "RIFF") && tag(8, "WEBP")) {
    let offset = 12;
    while (offset + 8 <= bytes.length) {
      const size = v.getUint32(offset + 4, true);
      const p = offset + 8;
      if (tag(offset, "VP8X") && size >= 10 && p + 10 <= bytes.length) {
        const u24 = (i: number) => bytes[i] | (bytes[i + 1] << 8) | (bytes[i + 2] << 16);
        return [1 + u24(p + 4), 1 + u24(p + 7)];
      }
      if (tag(offset, "VP8L") && size >= 5 && p + 5 <= bytes.length && bytes[p] === 47) {
        const bits = v.getUint32(p + 1, true);
        return [1 + (bits & 16383), 1 + ((bits >>> 14) & 16383)];
      }
      if (
        tag(offset, "VP8 ") &&
        size >= 10 &&
        p + 10 <= bytes.length &&
        tag(p + 3, "\x9d\x01\x2a")
      ) {
        return [v.getUint16(p + 6, true) & 16383, v.getUint16(p + 8, true) & 16383];
      }
      if (p + size > bytes.length) break;
      offset = p + size + (size & 1);
    }
  }
  throw new Error("Unsupported or incomplete image header. Re-export as PNG, JPEG or WebP.");
}
