#!/usr/bin/env bash
# Renders the component gallery and the main window's pages (Início, Estilo, Ajustes) in the four
# theme/mode combinations into shots/, in Portuguese. The Início gallery shows design/screens as
# its captures, never the user's own folder. Run after `cargo build --release`. Works in Git Bash,
# Linux and macOS.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
exe="$root/target/release/takeshot"
[ -x "$exe.exe" ] && exe="$exe.exe"
mkdir -p "$root/shots"
for page in componentes inicio estilo ajustes; do
  for theme in takeui coral; do
    for mode in dark light; do
      out="$root/shots/$page-$theme-$mode.png"
      rm -f "$out"
      "$exe" --page "$page" --theme "$theme" --mode "$mode" --lang pt \
        --captures-dir "$root/design/screens" --screenshot "$out"
      [ -f "$out" ] || { echo "screenshot missing: $out" >&2; exit 1; }
      echo "$out"
    done
  done
done
