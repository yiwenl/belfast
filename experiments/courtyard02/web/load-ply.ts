export async function loadPlyBytes(
  url: string,
  signal: AbortSignal,
  progress: (received: number, total: number) => void,
): Promise<Uint8Array> {
  const response = await fetch(url, { signal });
  if (!response.ok) throw new Error(`Unable to load PLY (${response.status})`);
  const total = Number(response.headers.get("content-length"));
  const encoding = response.headers.get("content-encoding");
  if (
    !response.body ||
    !Number.isSafeInteger(total) ||
    total <= 0 ||
    (encoding && encoding !== "identity")
  ) {
    // Fetch transparently decodes compressed bodies; Content-Length is then compressed size.
    return new Uint8Array(await response.arrayBuffer());
  }
  // One allocation for this large asset, avoiding a second copy to concatenate chunks.
  const bytes = new Uint8Array(total);
  const reader = response.body.getReader();
  let received = 0;
  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      if (received + value.byteLength > total)
        throw new Error("PLY download exceeds its declared size");
      bytes.set(value, received);
      received += value.byteLength;
      progress(received, total);
    }
    if (received !== total) throw new Error("PLY download was truncated");
    return bytes;
  } finally {
    reader.releaseLock();
  }
}
