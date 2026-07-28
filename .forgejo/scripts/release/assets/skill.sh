#!/usr/bin/env sh
set -eu

RELEASE_VERSION=${1:-}
ARTIFACT_DIR=${2:-}
[ -n "$RELEASE_VERSION" ] || { echo "missing release version" >&2; exit 1; }
[ -d "$ARTIFACT_DIR" ] || { echo "artifact dir missing: $ARTIFACT_DIR" >&2; exit 1; }

ROOT=$(CDPATH= cd -- "$(dirname "$0")/../../../.." && pwd)
NAME=plumb
SOURCE="$ROOT/skills/$NAME"
[ -f "$SOURCE/SKILL.md" ] || { echo "missing skill source: $SOURCE/SKILL.md" >&2; exit 1; }

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT INT TERM

cp -R "$SOURCE" "$tmpdir/$NAME"
cat > "$tmpdir/$NAME/metadata.json" <<META
{
  "schema": 1,
  "name": "$NAME",
  "version": "$RELEASE_VERSION",
  "keeper": "$NAME"
}
META

archive="$NAME-skill.tar.gz"
tar -C "$tmpdir" -czf "$ARTIFACT_DIR/$archive" "$NAME"
printf '%s\n' "$ARTIFACT_DIR/$archive"
