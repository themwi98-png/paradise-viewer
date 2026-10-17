#!/usr/bin/env bash
# Builds the folder (and zip) players download: the viewer's setup, the
# viewer app, and from the gta-paradise project next to this one the
# converter (bpx) and the added collision. It only needs Burnout Paradise
# Remastered. No game files, and no map edits (the viewer shows the GTA mod's
# when it's on the PC, stock Burnout otherwise).
#
#   tools/package.sh        -> dist/Paradise-Viewer-<version>/ and .zip
#
# The version is paradise-viewer-setup's (crates/paradise-viewer-setup/Cargo.toml).
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
GTA=../gta-paradise
[ -f "$GTA/crates/bpx/Cargo.toml" ] || { echo "needs the gta-paradise project next to this one ($GTA)"; exit 1; }

VERSION=$(grep -m1 '^version' crates/paradise-viewer-setup/Cargo.toml | cut -d'"' -f2)
NAME="Paradise-Viewer-$VERSION"
DIST="dist/$NAME"

# Release builds without this PC's folder names inside them (Rust records
# source paths for error messages). RUSTFLAGS replaces .cargo/config.toml's
# flags, so the static C runtime is asked for here too.
HOME_WIN=$(cygpath -w "$HOME")
CARGO_HOME_WIN=$(cygpath -w "${CARGO_HOME:-$HOME/.cargo}")
PROJECT_WIN=$(cygpath -w "$PWD")
GTA_WIN=$(cygpath -w "$(cd "$GTA" && pwd)")
export RUSTFLAGS="-C target-feature=+crt-static --remap-path-prefix=$CARGO_HOME_WIN=cargo --remap-path-prefix=$PROJECT_WIN=paradise-viewer --remap-path-prefix=$GTA_WIN=gta-paradise --remap-path-prefix=$HOME_WIN=home"
cargo build --release -q --target-dir target/dist -p paradise-viewer -p paradise-viewer-setup
cargo build --release -q --target-dir target/dist --manifest-path "$GTA/Cargo.toml" -p bpx

rm -rf "$DIST" "$DIST.zip"
mkdir -p "$DIST/tools" "$DIST/app" "$DIST/data"
cp target/dist/release/paradise-viewer-setup.exe "$DIST/"
cp target/dist/release/bpx.exe "$DIST/tools/"
cp target/dist/release/paradise-viewer.exe "$DIST/app/"
cp -r "$GTA/tools/collision" "$DIST/data/"
cp docs/PLAYER_README.txt "$DIST/README.txt"
cp LICENSE "$DIST/LICENSE.txt"
echo "$VERSION" > "$DIST/version.txt"

# Nothing personal inside: no Windows user name in any file (as a whole
# word, so a GitHub name that contains it, like in LICENSE, is fine).
USER_NAME=$(basename "$HOME")
if grep -rlaiw -- "$USER_NAME" "$DIST" >/dev/null; then
  echo "refusing: these files contain '$USER_NAME':"; grep -rlaiw -- "$USER_NAME" "$DIST"; exit 1
fi

(cd dist && powershell -NoProfile -Command "Compress-Archive -Path '$NAME' -DestinationPath '$NAME.zip' -Force")
du -sh "$DIST" "dist/$NAME.zip"
echo "packaged: dist/$NAME.zip"
