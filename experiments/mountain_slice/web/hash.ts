// fxhash base58 decoding and sfc32 adapted from fxhash-projects/_templateAlfrid/src/hash.js.
const alphabet = "123456789abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ";
export const DEFAULT_HASH = "ooXPar81Rntm6WRtG8qmQXja3fnrHQ9rAH4LL7dMHTDAgU7f5tL";

export function seededRandom(hash: string) {
  let words: number[];
  if (/^oo[1-9A-HJ-NP-Za-km-z]{49}$/.test(hash)) {
    words = hash
      .slice(2)
      .match(/.{12}/g)!
      .map((part) =>
        part
          .split("")
          .reduce((sum, c, i) => sum + alphabet.indexOf(c) * 58 ** (part.length - i - 1), 0),
      );
  } else {
    words = [0, 1, 2, 3].map((salt) => {
      let word = (2166136261 ^ salt) >>> 0;
      for (const c of hash) word = Math.imul(word ^ c.charCodeAt(0), 16777619) >>> 0;
      return word;
    });
  }
  let [a, b, c, d] = words;
  return () => {
    a |= 0;
    b |= 0;
    c |= 0;
    d |= 0;
    const t = (((a + b) | 0) + d) | 0;
    d = (d + 1) | 0;
    a = b ^ (b >>> 9);
    b = (c + (c << 3)) | 0;
    c = (c << 21) | (c >>> 11);
    c = (c + t) | 0;
    return (t >>> 0) / 4294967296;
  };
}

export function newHash() {
  return (
    "oo" + Array.from(crypto.getRandomValues(new Uint8Array(49)), (n) => alphabet[n % 58]).join("")
  );
}
