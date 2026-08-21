#!/usr/bin/env bash
set -euo pipefail

example="${1:?usage: $0 <example>}"
root="$(cd "$(dirname "$0")/.." && pwd)"
"$root/scripts/build-example-wasm.sh" "$example"

out="$root/dist/examples/${example}"
echo "Serving $example at http://127.0.0.1:8080"
cd "$out"
python3 -m http.server 8080
