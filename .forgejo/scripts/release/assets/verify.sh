#!/usr/bin/env sh
set -eu

MODE=${1:-}
RELEASE_VERSION=${2:-}
ARTIFACT_DIR=${3:-}
[ -n "$MODE" ] || { echo "missing mode" >&2; exit 1; }
[ -n "$RELEASE_VERSION" ] || { echo "missing release version" >&2; exit 1; }
[ -d "$ARTIFACT_DIR" ] || { echo "artifact dir missing: $ARTIFACT_DIR" >&2; exit 1; }

TARBALLS="plumb-x86_64-unknown-linux-gnu.tar.gz plumb-aarch64-apple-darwin.tar.gz plumb-x86_64-apple-darwin.tar.gz"
ASSETS="$TARBALLS plumb-skill.tar.gz plumb-x86_64-pc-windows-msvc.zip"

require() {
  [ -f "$ARTIFACT_DIR/$1" ] || { echo "missing artifact: $1" >&2; exit 1; }
}

checksum() {
  awk -v name="$1" 'NR > 1 && $NF == name { found = 1 } END { exit(found ? 0 : 1) }' \
    "$ARTIFACT_DIR/checksums.txt"
}

case "$MODE" in
  accept)
    require checksums.txt
    held=$(sed -n 's/^VERSION: *//p' "$ARTIFACT_DIR/checksums.txt" | head -n 1)
    [ "$held" = "$RELEASE_VERSION" ] || {
      echo "version mismatch: expected $RELEASE_VERSION got $held" >&2
      exit 1
    }
    for asset in $ASSETS; do
      require "$asset"
      checksum "$asset" || { echo "missing checksum entry: $asset" >&2; exit 1; }
    done
    ;;
  verify)
    (
      cd "$ARTIFACT_DIR"
      tail -n +2 checksums.txt > archive-checksums.txt
      if command -v sha256sum >/dev/null 2>&1; then
        sha256sum -c archive-checksums.txt
      else
        shasum -a 256 -c archive-checksums.txt
      fi
      rm -f archive-checksums.txt
    )
    for tarball in $TARBALLS; do
      tar tzf "$ARTIFACT_DIR/$tarball" | grep -Fxq plumb || {
        echo "missing plumb in $tarball" >&2
        exit 1
      }
    done
    python3 - "$ARTIFACT_DIR/plumb-x86_64-pc-windows-msvc.zip" <<'PY'
import sys
import zipfile

with zipfile.ZipFile(sys.argv[1]) as archive:
    if "plumb.exe" not in archive.namelist():
        raise SystemExit("missing plumb.exe in Windows archive")
PY
    ;;
  *)
    echo "unknown mode: $MODE" >&2
    exit 1
    ;;
esac
