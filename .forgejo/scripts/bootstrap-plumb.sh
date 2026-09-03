#!/bin/sh
set -eu

atom=${1:-.plumb-atom}
mode=${2:-bootstrap}
case "$mode" in
  bootstrap|exact) ;;
  *) printf 'unknown Plumb bootstrap mode: %s\n' "$mode" >&2; exit 2 ;;
esac
fetch='curl -fsSL --retry 5 --retry-all-errors --retry-delay 1 --connect-timeout 5 --max-time 30'
fetch_workload='curl -fsSL --retry 5 --retry-all-errors --retry-delay 1 --connect-timeout 5 --max-time 300'
manager="$RUNNER_TEMP/manage-plumb.sh"
$fetch -o "$manager" "https://releases.plumb.perish.uk/manage.sh"
held=$($fetch "https://releases.plumb.perish.uk/v1/channels/stable.json" 2>/dev/null \
  | sed -n 's/.*"releaseVersion"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p') || held=
if [ -n "$held" ]; then
  seat="$RUNNER_TEMP/plumb-$held"
  sh "$manager" install \
    --channel stable --version "$held" \
    --install-root "$seat/versions" --bin-dir "$seat/bin"
  bin="$seat/bin"
else
  sh "$manager"
  bin="$HOME/.local/bin"
fi
tool="$bin/plumb"
install_configuration() {
  if "$tool" configuration --help >/dev/null 2>&1; then
    "$tool" configuration install
  else
    printf 'installed Plumb has no configuration command; retaining its managed depot seat\n'
  fi
}
if [ "$mode" = bootstrap ]; then
  install_configuration
fi
: "${PLUMB_BUILD_VERSION:?PLUMB_BUILD_VERSION is required}"
: "${PLUMB_BUILD_COMMIT:?PLUMB_BUILD_COMMIT is required}"
if [ -z "${PLUMB_BUILD_CHANNEL:-}" ]; then
  case "$PLUMB_BUILD_VERSION" in
    *-alpha.*) PLUMB_BUILD_CHANNEL=alpha ;;
    *-beta.*) PLUMB_BUILD_CHANNEL=beta ;;
    *-rc.*) PLUMB_BUILD_CHANNEL=rc ;;
    *) PLUMB_BUILD_CHANNEL=stable ;;
  esac
fi
target="$RUNNER_TEMP/plumb-atom-$PLUMB_BUILD_COMMIT"
archive="$RUNNER_TEMP/plumb-atom-$PLUMB_BUILD_COMMIT.tgz"
host=$(rustc -vV | sed -n 's/^host: //p')
compiler=$(rustc --version)
install_atom() {
  held_source=$1
  digest=$(printf '%s' "$held_source" | sed -n 's#^.*/workloads/\([0-9a-fA-F]\{64\}\)\.tgz$#\1#p')
  test -n "$digest"
  $fetch_workload -o "$archive" "$held_source"
  actual=$(sha256sum "$archive" | cut -d' ' -f1)
  test "$actual" = "$digest"
  mkdir -p "$target/debug"
  tar -xzf "$archive" -C "$target/debug"
}
atom_plan() {
  "$tool" workflow plan \
    --world "target=$host" \
    --world "version=$PLUMB_BUILD_VERSION" \
    --world "channel=$PLUMB_BUILD_CHANNEL" \
    --workload "commit=$PLUMB_BUILD_COMMIT" \
    --world "compiler=$compiler" \
    --world 'profile=debug' \
    --root 'ship/atom=*' \
    --inventory-url "$PLUMB_WORKFLOW_INVENTORY_URL" \
    "$atom"
}
keys=
source=${PLUMB_ATOM_SOURCE:-}
supports_workload=
if "$tool" workflow plan --help 2>&1 | grep -q -- '--workload'; then
  supports_workload=1
fi
if [ -n "$source" ]; then
  install_atom "$source"
elif [ -n "${PLUMB_WORKFLOW_INVENTORY_URL:-}" ] && [ -n "$supports_workload" ]; then
  plan=$(atom_plan)
  keys=$(printf '%s' "$plan" | jq -c '.actions[] | select(.name == "ship/atom") | .keys')
  source=$(printf '%s' "$plan" | jq -r \
    '.actions[] | select(.name == "ship/atom" and .decision == "reuse" and .reuse.type == "workload") | .reuse.source')
fi
if [ -n "$source" ]; then
  install_atom "$source"
  printf 'reused exact Plumb atom %s for %s\n' "$PLUMB_BUILD_COMMIT" "$host"
else
  PLUMB_BUILD_SOURCE=1 CARGO_TARGET_DIR="$target" cargo build --quiet --locked --manifest-path "$atom/Cargo.toml" --bin plumb
  tar -czf "$archive" -C "$target/debug" plumb
  printf 'built exact Plumb atom %s for %s\n' "$PLUMB_BUILD_COMMIT" "$host"
fi
cp "$target/debug/plumb" "$tool"
printf '%s\n' "$bin" >> "$GITHUB_PATH"
"$tool" --version
if [ "$mode" = exact ]; then
  install_configuration
fi
if [ -z "$keys" ] && [ -n "${PLUMB_WORKFLOW_INVENTORY_URL:-}" ]; then
  plan=$(atom_plan)
  keys=$(printf '%s' "$plan" | jq -c '.actions[] | select(.name == "ship/atom") | .keys')
fi
if [ -z "$source" ] && [ -n "$keys" ]; then
  if ! "$tool" workflow record ship/atom --keys "$keys" --workload "$archive"; then
    winner=
    attempt=1
    while [ "$attempt" -le 12 ]; do
      raced=$(atom_plan)
      winner=$(printf '%s' "$raced" | jq -r \
        '.actions[] | select(.name == "ship/atom" and .decision == "reuse" and .reuse.type == "workload") | .reuse.source')
      [ -n "$winner" ] && break
      [ "$attempt" -lt 12 ] || break
      printf 'waiting for exact Plumb atom inventory winner (%s/12)\n' "$attempt"
      sleep 5
      attempt=$((attempt + 1))
    done
    test -n "$winner"
    source=$winner
    install_atom "$winner"
    cp "$target/debug/plumb" "$tool"
    printf 'accepted exact Plumb atom inventory winner %s for %s\n' "$winner" "$host"
  else
    digest=$(sha256sum "$archive" | cut -d' ' -f1)
    source="${PLUMB_WORKFLOW_INVENTORY_URL%/}/workloads/$digest.tgz"
  fi
fi
if [ -n "$source" ] && [ -n "${GITHUB_OUTPUT:-}" ]; then
  printf 'source=%s\n' "$source" >> "$GITHUB_OUTPUT"
fi
if [ -n "$source" ] && [ -n "${GITHUB_ENV:-}" ]; then
  printf 'PLUMB_ATOM_SOURCE=%s\n' "$source" >> "$GITHUB_ENV"
fi
