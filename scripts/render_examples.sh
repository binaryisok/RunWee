#!/usr/bin/env bash
# Renders the example gallery (used by CI). Renders go to ./renders.
set -euo pipefail
mkdir -p renders
QUALITY="${QUALITY:-low}"
shopt -s nullglob
for f in examples/scripts/*.rhai; do
  name=$(basename "$f" .rhai)
  echo "== rendering $name"
  ./target/release/runwee render "$f" --quality "$QUALITY" -o "renders/$name.mp4"
done
for ex in $(ls crates/runwee/examples/*.rs 2>/dev/null | xargs -n1 basename 2>/dev/null | sed 's/\.rs$//'); do
  echo "== rust example $ex"
  RUNWEE_QUALITY="$QUALITY" RUNWEE_OUT="renders/$ex.mp4" cargo run --release -q -p runwee --example "$ex"
done
ls -la renders
