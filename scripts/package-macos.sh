#!/bin/bash
set -euo pipefail
# Maintainer build only; users receive the composed archive, not a compiler step.
root=$(cd "$(dirname "$0")/.." && pwd)
source_dir=$(cd "${1:?provide a clean pinned ds4 checkout}" && pwd)
revision=0aaea5a238fb41a35106a551e73c8409dfb751ac
[[ $(uname -s) == Darwin && $(uname -m) == arm64 ]]
[[ $(git -C "$source_dir" rev-parse HEAD) == "$revision" ]]
[[ -z $(git -C "$source_dir" status --porcelain) ]]
cd "$root"
mkdir -p dist
stage=$(mktemp -d "$root/dist/package.XXXXXX")
trap 'rm -rf "$stage"; make -C "$source_dir" clean >/dev/null' EXIT
# Do not bake the maintainer CPU generation into the portable Apple Silicon bundle.
make -C "$source_dir" -j4 ds4-server NATIVE_CPU_FLAG=-mcpu=apple-m1 DEBUG_FLAGS=
cargo build --locked --release
mkdir -p "$stage/ds4/runtime"
cp target/release/ds4 plugin.toml README.md LICENSE "$stage/ds4/"
cp "$source_dir/ds4-server" "$stage/ds4/runtime/"
cp -R "$source_dir/metal" "$stage/ds4/runtime/"
cp "$source_dir/LICENSE" "$stage/ds4/runtime/LICENSE"
printf '%s\n' "$revision" > "$stage/ds4/runtime/UPSTREAM_REVISION"
codesign --force --sign - "$stage/ds4/ds4"
codesign --force --sign - "$stage/ds4/runtime/ds4-server"
codesign --verify --strict "$stage/ds4/ds4"
codesign --verify --strict "$stage/ds4/runtime/ds4-server"
"$stage/ds4/ds4" catalog
"$stage/ds4/runtime/ds4-server" --help >/dev/null
(cd "$stage/ds4" && find runtime -type f -exec shasum -a 256 {} \; > RUNTIME.sha256)
archive="$root/dist/ds4-v0.1.0-aarch64-apple-darwin.tar.gz"
tar -czf "$archive" -C "$stage" ds4
(cd dist && shasum -a 256 "$(basename "$archive")" > "$(basename "$archive").sha256")
printf 'Trial archive: %s\n' "$archive"
