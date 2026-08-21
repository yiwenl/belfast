#!/usr/bin/env bash
set -euo pipefail

example="${1:?usage: $0 <example>}"
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
target_dir="$(
  python3 - <<'PY'
import json, subprocess
meta = json.loads(subprocess.check_output(["cargo", "metadata", "--format-version", "1", "--no-deps"]))
print(meta["target_directory"])
PY
)"
out="$target_dir/example-wasm"
wasm="$target_dir/wasm32-unknown-unknown/release/examples/${example}.wasm"

bindgen_version="$(
  python3 - "$root/Cargo.lock" <<'PY'
import pathlib, re, sys
text = pathlib.Path(sys.argv[1]).read_text()
match = re.search(r'name = "wasm-bindgen"\nversion = "([^"]+)"', text)
if not match:
    raise SystemExit("wasm-bindgen version missing from Cargo.lock")
print(match.group(1))
PY
)"

resolve_wasm_bindgen() {
  if command -v wasm-bindgen >/dev/null 2>&1; then
    command -v wasm-bindgen
    return
  fi
  local candidate
  for candidate in \
    "$HOME/Library/Caches/.wasm-pack/wasm-bindgen-cargo-install-${bindgen_version}/wasm-bindgen" \
    "$HOME/.cache/.wasm-pack/wasm-bindgen-cargo-install-${bindgen_version}/wasm-bindgen"
  do
    if [[ -x "$candidate" ]]; then
      echo "$candidate"
      return
    fi
  done
  echo "error: wasm-bindgen ${bindgen_version} not found. Install with:" >&2
  echo "  cargo install wasm-bindgen-cli --version ${bindgen_version}" >&2
  echo "or run a wasm-pack build once so it caches that CLI." >&2
  exit 1
}

wasm_bindgen_bin="$(resolve_wasm_bindgen)"

cargo build -p belfast --example "$example" --target wasm32-unknown-unknown --release
"$wasm_bindgen_bin" "$wasm" --target web --out-dir "$out" --out-name index
cp "$root/crates/belfast/examples/web/index.html" "$out/index.html"

echo "Serving $example at http://127.0.0.1:8080"
cd "$out"
python3 -m http.server 8080
