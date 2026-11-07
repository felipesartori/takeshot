#!/usr/bin/env bash
# Live preview: rebuilds and reopens Takeshot whenever src/, assets/ or Cargo.toml change.
# Usage: scripts/dev.sh [takeshot arguments]   (default: --page componentes)
# Builds into target/dev-watch so it never waits on the lock of other cargo runs.
set -uo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
export CARGO_TARGET_DIR="$root/target/dev-watch"
if [ $# -eq 0 ]; then set -- --page componentes; fi
log="$CARGO_TARGET_DIR/dev-build.log"
marker="$CARGO_TARGET_DIR/dev-marker"
mkdir -p "$CARGO_TARGET_DIR"
pid=""

stop_app() {
  [ -n "$pid" ] || return 0
  if [ -r "/proc/$pid/winpid" ]; then
    taskkill //F //PID "$(cat "/proc/$pid/winpid")" >/dev/null 2>&1
  else
    kill "$pid" 2>/dev/null
  fi
  wait "$pid" 2>/dev/null
  pid=""
}
trap 'stop_app; exit 0' INT TERM

build_and_run() {
  echo "[dev] $(date +%H:%M:%S) compilando..."
  if cargo build --manifest-path "$root/Cargo.toml" >"$log" 2>&1; then
    exe="$CARGO_TARGET_DIR/debug/takeshot"
    [ -f "$exe.exe" ] && exe="$exe.exe"
    # Windows locks a running executable, so the window runs a copy and the build stays free to replace the original.
    run="$CARGO_TARGET_DIR/run/$(basename "$exe")"
    stop_app
    mkdir -p "$(dirname "$run")"
    cp "$exe" "$run"
    "$run" "$@" &
    pid=$!
    echo "[dev] $(date +%H:%M:%S) aberto: takeshot $*"
  else
    echo "[dev] $(date +%H:%M:%S) erro de compilação; a janela anterior continua aberta. Detalhes:"
    grep -E "^(error|warning)" -A6 "$log" | head -40
  fi
}

touch "$marker"
build_and_run "$@"
while true; do
  sleep 1
  changed="$(find "$root/src" "$root/assets" "$root/Cargo.toml" -type f -newer "$marker" 2>/dev/null | head -1)"
  if [ -n "$changed" ]; then
    touch "$marker"
    build_and_run "$@"
  fi
done
