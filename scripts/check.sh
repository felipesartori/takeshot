#!/usr/bin/env bash
# What CI checks, before a push: fmt, clippy and tests on this machine, then the same on Linux in a
# container (scripts/ci-linux) plus clippy for macOS, on the exact commit being pushed. Code behind
# #[cfg(...)] for another system only compiles there, so a host-only check misses it.
#
#   scripts/check.sh                 host (working tree) + Linux/macOS (HEAD)
#   scripts/check.sh --rev <commit>  Linux/macOS check that commit
#   scripts/check.sh --host | --linux
#
# Logs go to shots/check-*.log. The pre-push hook (.githooks/pre-push) runs it.
set -u
cd "$(git rev-parse --show-toplevel)" || exit 1
mkdir -p shots

rev=HEAD
run_host=1
run_linux=1
while [ $# -gt 0 ]; do
  case "$1" in
    --rev) rev="$2"; shift ;;
    --host) run_linux=0 ;;
    --linux) run_host=0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
  shift
done

fail() {
  echo "check: $1 failed, see $2" >&2
  exit 1
}

if [ "$run_host" = 1 ]; then
  log=shots/check-host.log
  echo "check: host (fmt, clippy, tests)..."
  {
    cargo fmt --check &&
      cargo clippy --all-targets -- -D warnings &&
      cargo test
  } > "$log" 2>&1 || fail "host" "$log"
fi

if [ "$run_linux" = 1 ]; then
  sha=$(git rev-parse --verify "$rev^{commit}") || exit 1
  if ! docker info > /dev/null 2>&1; then
    echo "check: Docker is not running; start Docker Desktop (the Linux and macOS check needs it)." >&2
    exit 1
  fi
  image=takeshot-ci-linux
  # Rebuilt when the Dockerfile changes: its hash is part of the tag.
  tag="$image:$(git hash-object scripts/ci-linux/Dockerfile | cut -c1-12)"
  if ! docker image inspect "$tag" > /dev/null 2>&1; then
    echo "check: building $tag (first time takes a few minutes)..."
    docker build -t "$tag" scripts/ci-linux > shots/check-image.log 2>&1 ||
      fail "image build" shots/check-image.log
  fi
  log=shots/check-linux.log
  echo "check: Linux (fmt, clippy, tests) and macOS (clippy) on ${sha:0:7}..."
  # tar -m: fresh mtimes, so cargo never takes an older commit's files for already built.
  # Named volumes keep the crates, the build and rustup between runs.
  git archive --format=tar "$sha" |
    MSYS_NO_PATHCONV=1 docker run --rm -i \
      -v takeshot-ci-target:/src/target \
      -v takeshot-ci-registry:/usr/local/cargo/registry \
      -v takeshot-ci-rustup:/usr/local/rustup \
      "$tag" bash -c '
        set -e
        tar -x -m -f -
        rustup update stable --no-self-update > /dev/null 2>&1 || echo "rustup update failed; using the toolchain in the image"
        rustup target add aarch64-apple-darwin > /dev/null 2>&1 || true
        echo "== $(rustc --version)"
        echo "== linux: fmt";    cargo fmt --check
        echo "== linux: clippy"; cargo clippy --all-targets -- -D warnings
        echo "== linux: test";   cargo test
        echo "== macos: clippy"; cargo clippy --all-targets --target aarch64-apple-darwin -- -D warnings
      ' > "$log" 2>&1 || fail "Linux/macOS" "$log"
fi

echo "check: ok"
