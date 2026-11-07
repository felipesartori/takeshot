#!/usr/bin/env bash
# Renders the component gallery in the four theme/mode combinations into shots/.
# Run after `cargo build --release`. Works in Git Bash, Linux and macOS.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
exe="$root/target/release/takeshot"
[ -x "$exe.exe" ] && exe="$exe.exe"
mkdir -p "$root/shots"
for theme in takeui coral; do
  for mode in dark light; do
    out="$root/shots/componentes-$theme-$mode.png"
    rm -f "$out"
    "$exe" --page componentes --theme "$theme" --mode "$mode" --lang pt --screenshot "$out"
    [ -f "$out" ] || { echo "screenshot missing: $out" >&2; exit 1; }
    echo "$out"
  done
done
