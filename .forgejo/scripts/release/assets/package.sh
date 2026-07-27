#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname "$0")/../../../.." && pwd)
NAME=plumb
VERSION=$(sed -n 's/^version = "\(.*\)"$/\1/p' "$ROOT/Cargo.toml" | head -n 1)
RELEASE_VERSION=${1:-${RELEASE_VERSION:-v$VERSION}}
TARGET=${TARGET:-$(rustc -Vv | sed -n 's/^host: //p')}
DIST_DIR=${DIST_DIR:-"$ROOT/dist"}
ARTIFACT_DIR="$DIST_DIR/$RELEASE_VERSION"

mkdir -p "$ARTIFACT_DIR"
PLUMB_BUILD_VERSION="$RELEASE_VERSION" \
  cargo build --release --locked -p plumb-cli --target "$TARGET"

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT INT TERM
cp "$ROOT/target/$TARGET/release/$NAME" "$tmpdir/$NAME"
chmod +x "$tmpdir/$NAME"

archive="$NAME-$TARGET.tar.gz"
tar -C "$tmpdir" -czf "$ARTIFACT_DIR/$archive" "$NAME"
printf '%s\n' "$ARTIFACT_DIR/$archive"
