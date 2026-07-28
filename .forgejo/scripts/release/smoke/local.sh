#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname "$0")/../../../.." && pwd)
VERSION=v0.4.0-beta.1
TARGET=$(rustc -Vv | sed -n 's/^host: //p')
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT INT TERM
cd "$ROOT"

DIST_DIR="$fixture/assets" TARGET="$TARGET" \
  sh "$ROOT/.forgejo/scripts/release/assets/package.sh" "$VERSION"
archive="plumb-$TARGET.tar.gz"
release="$fixture/releases"
seat="$release/stable/versions/$VERSION"
mkdir -p "$seat" "$release/stable/latest"
cp "$fixture/assets/$VERSION/$archive" "$seat/$archive"
printf '{"releaseVersion":"%s"}\n' "$VERSION" > "$release/stable/latest/metadata.json"

cp "$seat/$archive" "$fixture/assets/$VERSION/plumb-aarch64-apple-darwin.tar.gz"
cp "$seat/$archive" "$fixture/assets/$VERSION/plumb-x86_64-apple-darwin.tar.gz"
python3 - "$fixture/assets/$VERSION/plumb-x86_64-pc-windows-msvc.zip" <<'PY'
import sys
import zipfile

with zipfile.ZipFile(sys.argv[1], "w") as archive:
    archive.writestr("plumb.exe", b"fixture")
PY
sh .forgejo/scripts/release/assets/skill.sh "$VERSION" "$fixture/assets/$VERSION"
sh .forgejo/scripts/release/assets/checksums.sh "$VERSION" "$fixture/assets/$VERSION"
sh .forgejo/scripts/release/assets/verify.sh accept "$VERSION" "$fixture/assets/$VERSION"
sh .forgejo/scripts/release/assets/verify.sh verify "$VERSION" "$fixture/assets/$VERSION"
cp "$fixture/assets/$VERSION/checksums.txt" "$seat/checksums.txt"

mkdir -p "$fixture/bin"
printf '#!/usr/bin/env sh\nexit 0\n' > "$fixture/bin/aws"
chmod +x "$fixture/bin/aws"
PATH="$fixture/bin:$PATH" \
PLUMB_RELEASES_PUBLIC_URL=https://releases.example.test \
PLUMB_RELEASES_S3_AK=fixture \
PLUMB_RELEASES_S3_SK=fixture \
PLUMB_RELEASES_S3_BUCKET=fixture \
PLUMB_RELEASES_S3_URL=https://s3.example.test \
RELEASE_CHANNEL=beta \
RELEASE_VERSION="$VERSION" \
RELEASE_ROOT="$fixture/assets/$VERSION" \
BASE_VERSION=0.4.0 \
BETA_NUMBER=1 \
CI_REPOSITORY=PerishLab/plumb \
CI_COMMIT=fixture \
  bash .forgejo/scripts/release/r2/publish.sh
jq -e \
  --arg version "$VERSION" \
  '.releaseVersion == $version
    and .betaVersion == $version
    and .baseVersion == "0.4.0"
    and .betaNumber == 1
    and (.artifacts | length == 6)
    and (.artifacts.skillTarGz.sha256 | length == 64)' \
  "$fixture/assets/$VERSION/metadata.json" >/dev/null

export HOME="$fixture/home"
export PLUMB_INSTALL_ROOT="$fixture/install"
export PLUMB_LOCAL_BIN_DIR="$fixture/local-bin"
mkdir -p "$HOME"
if sh "$ROOT/manage.sh" install --channel nightly >/dev/null 2>&1; then
  echo "manager accepted an invalid channel" >&2
  exit 1
fi
if sh "$ROOT/manage.sh" uninstall --version ../../escape >/dev/null 2>&1; then
  echo "manager accepted an invalid version" >&2
  exit 1
fi
sh "$ROOT/manage.sh" install --public-url "file://$release"
"$PLUMB_LOCAL_BIN_DIR/plumb" --version | grep -F "$VERSION"
"$PLUMB_LOCAL_BIN_DIR/plumb" doctor "$ROOT"
sh "$ROOT/manage.sh" update --public-url "file://$release" --version "$VERSION"
sh "$ROOT/manage.sh" uninstall --version "$VERSION"
[ ! -e "$PLUMB_INSTALL_ROOT/$VERSION" ] || {
  echo "local smoke left $PLUMB_INSTALL_ROOT/$VERSION" >&2
  exit 1
}
