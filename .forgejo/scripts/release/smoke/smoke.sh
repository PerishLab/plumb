#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname "$0")/../../../.." && pwd)
VERSION=${1:-}
CHANNEL=${2:-stable}
[ -n "$VERSION" ] || { echo "missing release version" >&2; exit 1; }

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT INT TERM
export HOME="$tmpdir/home"
export PLUMB_INSTALL_ROOT="$tmpdir/install"
export PLUMB_LOCAL_BIN_DIR="$tmpdir/bin"
mkdir -p "$HOME" "$PLUMB_INSTALL_ROOT" "$PLUMB_LOCAL_BIN_DIR"

sh "$ROOT/manage.sh" install --channel "$CHANNEL" --version "$VERSION"
"$PLUMB_LOCAL_BIN_DIR/plumb" --version
"$PLUMB_LOCAL_BIN_DIR/plumb" doctor "$ROOT"
sh "$ROOT/manage.sh" update --channel "$CHANNEL" --version "$VERSION"
"$PLUMB_LOCAL_BIN_DIR/plumb" doctor "$ROOT"
sh "$ROOT/manage.sh" uninstall --version "$VERSION"
[ ! -e "$PLUMB_INSTALL_ROOT/$VERSION" ] || {
  echo "version uninstall left $PLUMB_INSTALL_ROOT/$VERSION" >&2
  exit 1
}

if [ "${SMOKE_LATEST:-}" = "1" ]; then
  if [ "$CHANNEL" = stable ]; then
    sh "$ROOT/manage.sh" install --channel "$CHANNEL"
    "$PLUMB_LOCAL_BIN_DIR/plumb" doctor "$ROOT"
    sh "$ROOT/manage.sh" uninstall
    [ ! -e "$PLUMB_INSTALL_ROOT" ] || {
      echo "full uninstall left $PLUMB_INSTALL_ROOT" >&2
      exit 1
    }
  elif sh "$ROOT/manage.sh" install --channel "$CHANNEL" >/dev/null 2>&1; then
    echo "manager accepted floating $CHANNEL install" >&2
    exit 1
  fi
fi
