#!/usr/bin/env sh
set -eu

COMMAND=${1:-install}
[ $# -gt 0 ] && shift || true

CHANNEL=${PLUMB_CHANNEL:-stable}
VERSION=${PLUMB_VERSION:-}
PUBLIC_URL=${PLUMB_RELEASES_PUBLIC_URL:-https://releases.plumb.perish.uk}
INSTALL_ROOT=${PLUMB_INSTALL_ROOT:-"$HOME/.local/share/plumb"}
LOCAL_BIN_DIR=${PLUMB_LOCAL_BIN_DIR:-"$HOME/.local/bin"}

usage() {
  cat <<'EOF'
plumb manager

Usage:
  manage.sh --help
  manage.sh install [--channel stable|beta] [--version vX.Y.Z[-beta.N]]
  manage.sh update [--channel stable|beta] [--version vX.Y.Z[-beta.N]]
  manage.sh uninstall [--version vX.Y.Z[-beta.N]]

install and update leave exactly one version on disk. Whatever was installed
before is swept once the new binary is linked and answers --version. Rolling
back is install --version <older>, which fetches that version again; released
artifacts are immutable and always retrievable, so nothing is lost by not
hoarding them.

Options:
  --public-url <url>     release metadata and artifact base URL
  --install-root <path>  versioned install root
  --bin-dir <path>       directory for the plumb link

Environment:
  PLUMB_RELEASES_PUBLIC_URL
  PLUMB_CHANNEL
  PLUMB_VERSION
  PLUMB_INSTALL_ROOT
  PLUMB_LOCAL_BIN_DIR
EOF
}

case "$COMMAND" in
  -h|--help|help) usage; exit 0 ;;
esac

while [ $# -gt 0 ]; do
  case "$1" in
    --channel) CHANNEL=${2:-}; shift 2 ;;
    --channel=*) CHANNEL=${1#--channel=}; shift ;;
    --version) VERSION=${2:-}; shift 2 ;;
    --version=*) VERSION=${1#--version=}; shift ;;
    --public-url) PUBLIC_URL=${2:-}; shift 2 ;;
    --public-url=*) PUBLIC_URL=${1#--public-url=}; shift ;;
    --install-root) INSTALL_ROOT=${2:-}; shift 2 ;;
    --install-root=*) INSTALL_ROOT=${1#--install-root=}; shift ;;
    --bin-dir) LOCAL_BIN_DIR=${2:-}; shift 2 ;;
    --bin-dir=*) LOCAL_BIN_DIR=${1#--bin-dir=}; shift ;;
    -h|--help|help) usage; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 1 ;;
  esac
done

for value in "$CHANNEL" "$PUBLIC_URL" "$INSTALL_ROOT" "$LOCAL_BIN_DIR"; do
  [ -n "$value" ] || { echo "manager option cannot be empty" >&2; exit 1; }
done
case "$CHANNEL" in
  stable|beta) ;;
  *) echo "invalid channel: $CHANNEL" >&2; exit 1 ;;
esac
case "$COMMAND:$CHANNEL:$VERSION" in
  install:stable:*|update:stable:*|uninstall:*) ;;
  install:*:|update:*:)
    echo "non-stable channel $CHANNEL requires an exact version" >&2
    exit 1
    ;;
esac
PUBLIC_URL=${PUBLIC_URL%/}

normalize_version() {
  normalized=$(printf 'v%s' "$(printf '%s' "$1" | sed 's/^v//')")
  printf '%s\n' "$normalized" |
    grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+(-beta\.[1-9][0-9]*)?$' || {
      echo "invalid plumb version: $1" >&2
      exit 1
    }
  printf '%s' "$normalized"
}

archive() {
  case "$(uname -s):$(uname -m)" in
    Linux:x86_64|Linux:amd64) echo plumb-x86_64-unknown-linux-gnu.tar.gz ;;
    Darwin:arm64|Darwin:aarch64) echo plumb-aarch64-apple-darwin.tar.gz ;;
    Darwin:x86_64|Darwin:amd64) echo plumb-x86_64-apple-darwin.tar.gz ;;
    *) echo "unsupported platform: $(uname -s) $(uname -m)" >&2; exit 1 ;;
  esac
}

latest() {
  sed -n 's/.*"releaseVersion"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$1" |
    head -n 1
}

install_plumb() {
  tmpdir=$(mktemp -d)
  trap 'rm -rf "$tmpdir"' EXIT INT TERM
  if [ -z "$VERSION" ]; then
    curl -fsSL "$PUBLIC_URL/$CHANNEL/latest/metadata.json" -o "$tmpdir/metadata.json"
    VERSION=$(latest "$tmpdir/metadata.json")
    [ -n "$VERSION" ] || { echo "failed to resolve latest plumb version" >&2; exit 1; }
  fi
  VERSION=$(normalize_version "$VERSION")
  case "$CHANNEL:$VERSION" in
    stable:v*-*) echo "stable channel cannot install prerelease $VERSION" >&2; exit 1 ;;
    beta:v*-beta.[1-9]*) ;;
    beta:*) echo "version $VERSION does not belong to beta" >&2; exit 1 ;;
  esac
  name=$(archive)
  prefix="$PUBLIC_URL/$CHANNEL/versions/$VERSION"
  curl -fsSL "$prefix/$name" -o "$tmpdir/$name"
  curl -fsSL "$prefix/checksums.txt" -o "$tmpdir/checksums.txt"
  awk -v name="$name" '$NF == name { print }' "$tmpdir/checksums.txt" > "$tmpdir/archive.sha"
  [ -s "$tmpdir/archive.sha" ] || { echo "no checksum for $name" >&2; exit 1; }
  (
    cd "$tmpdir"
    if command -v sha256sum >/dev/null 2>&1; then
      sha256sum -c archive.sha
    else
      shasum -a 256 -c archive.sha
    fi
  )

  rm -rf "$INSTALL_ROOT/$VERSION"
  mkdir -p "$INSTALL_ROOT/$VERSION" "$LOCAL_BIN_DIR"
  tar -xzf "$tmpdir/$name" -C "$INSTALL_ROOT/$VERSION"
  chmod +x "$INSTALL_ROOT/$VERSION/plumb"
  rm -f "$LOCAL_BIN_DIR/plumb"
  ln -s "$INSTALL_ROOT/$VERSION/plumb" "$LOCAL_BIN_DIR/plumb"
  "$LOCAL_BIN_DIR/plumb" --version
  printf 'installed plumb to %s\n' "$LOCAL_BIN_DIR/plumb"
  sweep
}

sweep() {
  swept=""
  for seat in "$INSTALL_ROOT"/*; do
    [ -d "$seat" ] || continue
    held=$(basename "$seat")
    if [ "$held" != "$VERSION" ]; then
      rm -rf "$seat"
      swept="$swept $held"
    fi
  done
  if [ -n "$swept" ]; then
    printf 'swept:%s\n' "$swept"
  fi
}

uninstall_plumb() {
  link="$LOCAL_BIN_DIR/plumb"
  if [ -n "$VERSION" ]; then
    VERSION=$(normalize_version "$VERSION")
    target="$INSTALL_ROOT/$VERSION/plumb"
    if [ -L "$link" ] && [ "$(readlink "$link" || true)" = "$target" ]; then
      rm -f "$link"
    fi
    rm -rf "$INSTALL_ROOT/$VERSION"
    rmdir "$INSTALL_ROOT" 2>/dev/null || true
    printf 'removed plumb %s\n' "$VERSION"
    return
  fi
  rm -f "$link"
  rm -rf "$INSTALL_ROOT"
  rmdir "$LOCAL_BIN_DIR" 2>/dev/null || true
  printf 'removed plumb\n'
}

case "$COMMAND" in
  install|update) install_plumb ;;
  uninstall) uninstall_plumb ;;
  *) echo "unknown command: $COMMAND" >&2; exit 1 ;;
esac
