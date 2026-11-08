#!/usr/bin/env bash
# The Linux packages (.deb and .AppImage) and the tar.gz, built in the CI image (scripts/ci-linux)
# from a commit, into target/packages-linux. Also prints the shared-library packages the binary
# needs (dpkg-shlibdeps), for [package.metadata.packager.deb] depends.
#
#   scripts/linux-packages.sh [commit]   (default HEAD)
set -eu
cd "$(git rev-parse --show-toplevel)"
sha=$(git rev-parse --verify "${1:-HEAD}^{commit}")
image=takeshot-ci-linux
tag="$image:$(git hash-object scripts/ci-linux/Dockerfile | cut -c1-12)"
if ! docker image inspect "$tag" > /dev/null 2>&1; then
  docker build -t "$tag" scripts/ci-linux
fi
out=target/packages-linux
rm -rf "$out"
mkdir -p "$out"
packager_version=$(grep -m1 'PACKAGER_VERSION:' .github/workflows/build.yml | awk '{print $2}')
git archive --format=tar "$sha" |
  MSYS_NO_PATHCONV=1 docker run --rm -i \
    -v takeshot-ci-target:/src/target \
    -v takeshot-ci-registry:/usr/local/cargo/registry \
    -v takeshot-ci-rustup:/usr/local/rustup \
    -v takeshot-ci-tools:/opt/tools \
    -v "$(pwd -W 2> /dev/null || pwd)/$out:/out" \
    -e APPIMAGE_EXTRACT_AND_RUN=1 \
    "$tag" bash -c "
      set -e
      tar -x -m -f -
      export PATH=/opt/tools/bin:\$PATH
      apt-get update -qq > /dev/null && apt-get install -y -qq --no-install-recommends file dpkg-dev > /dev/null
      cargo packager --version 2> /dev/null | grep -q '$packager_version' ||
        cargo install cargo-packager --version '$packager_version' --locked --root /opt/tools
      cargo build --release
      echo '== dpkg-shlibdeps'
      mkdir -p /tmp/shl/debian && cd /tmp/shl && printf 'Source: takeshot\n' > debian/control &&
        dpkg-shlibdeps -O /src/target/release/takeshot 2> /dev/null; cd /src
      rm -rf target/packages
      cargo packager --release --formats deb,appimage
      version=\$(grep -m1 '^version' Cargo.toml | cut -d'\"' -f2)
      cp target/packages/*.deb \"/out/takeshot-\$version-linux-x86_64.deb\"
      cp target/packages/*.AppImage \"/out/takeshot-\$version-linux-x86_64.AppImage\"
      tar -czf \"/out/takeshot-\$version-linux-x86_64.tar.gz\" -C target/release takeshot -C /src LICENSE THIRD_PARTY_NOTICES.md
      ls -la /out
    "
