# The Linux job of .github/workflows/build.yml, as an image: same Ubuntu, same libraries, plain
# stable Rust, plus the macOS target so clippy can check the macOS code (it only type-checks, no
# linking). scripts/check.sh builds and runs it.
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      ca-certificates curl build-essential \
      pkg-config clang libclang-dev libgtk-3-dev libxcb1-dev libxcb-randr0-dev libxcb-shm0-dev \
      libxrandr-dev libx11-dev libxkbcommon-dev libwayland-dev libdbus-1-dev libpipewire-0.3-dev \
      libegl1-mesa-dev libgbm-dev libgl1-mesa-dev \
 && rm -rf /var/lib/apt/lists/*

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH \
    RUSTUP_TOOLCHAIN=stable \
    CARGO_TERM_COLOR=never
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path --profile minimal \
      --default-toolchain stable --component clippy,rustfmt \
 && rustup target add aarch64-apple-darwin

WORKDIR /src
