#!/bin/sh
set -eu

atom=${1:-.plumb-atom}
mode=${2:-bootstrap}
configuration=${3:-${PLUMB_BUILD_VERSION:-}}
case "$mode" in
  bootstrap|exact) ;;
  *) printf 'unknown Plumb bootstrap mode: %s\n' "$mode" >&2; exit 2 ;;
esac
fetch='curl -fsSL --retry 5 --retry-all-errors --retry-delay 1 --connect-timeout 5 --max-time 30'
fetch_workload='curl -fsSL --retry 30 --retry-all-errors --retry-delay 2 --connect-timeout 5 --max-time 300'
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
bin="$RUNNER_TEMP/plumb-exact-$PLUMB_BUILD_COMMIT/bin"
tool="$bin/plumb"
host=$(rustc -vV | sed -n 's/^host: //p')
compiler=$(rustc --version)
install_configuration() {
  if "$tool" configuration --help >/dev/null 2>&1; then
    test -n "$configuration"
    if [ "$#" -eq 1 ]; then
      "$tool" configuration install --version "$configuration" --path "$1"
    else
      "$tool" configuration install --version "$configuration"
    fi
  else
    printf 'installed Plumb has no configuration command; retaining its managed depot seat\n'
  fi
}
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
handoff=${PLUMB_ATOM_HANDOFF:-}
inventory_base=${PLUMB_WORKFLOW_INVENTORY_URL%/}
inventory_base=${inventory_base%/inventory.json}
if [ -z "$source" ] && [ -n "$handoff" ]; then
  test "$(printf '%s' "$handoff" | jq -r '.type')" = workload
  source=$(printf '%s' "$handoff" | jq -r '.source')
  test -n "$source"
fi
supports_workload=
if [ -z "$source" ]; then
  manager="$RUNNER_TEMP/manage-plumb.sh"
  held=
  if $fetch -o "$manager" "https://releases.plumb.perish.uk/manage.sh"; then
    held=$($fetch "https://releases.plumb.perish.uk/v1/channels/stable.json" 2>/dev/null \
      | sed -n 's/.*"releaseVersion"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p') || held=
    seat="$RUNNER_TEMP/plumb-bootstrap-${held:-stable}"
    if [ -n "$held" ] && sh "$manager" install \
      --channel stable --version "$held" \
      --install-root "$seat/versions" --bin-dir "$seat/bin"; then
      tool="$seat/bin/plumb"
      printf 'installed stable Plumb %s for atom planning\n' "$held"
    elif [ -z "$held" ] && sh "$manager" install \
      --install-root "$seat/versions" --bin-dir "$seat/bin"; then
      tool="$seat/bin/plumb"
      printf 'installed stable Plumb for atom planning\n'
    else
      printf 'stable Plumb is unavailable; cold-building the exact atom\n'
    fi
  else
    printf 'stable Plumb manager is unavailable; cold-building the exact atom\n'
  fi
fi
if [ -x "$tool" ] && "$tool" workflow plan --help 2>&1 | grep -q -- '--workload'; then
  supports_workload=1
fi
if [ -z "$source" ] && [ -n "${PLUMB_WORKFLOW_INVENTORY_URL:-}" ] && [ -n "$supports_workload" ]; then
  plan=$(atom_plan)
  keys=$(printf '%s' "$plan" | jq -c '.actions[] | select(.name == "ship/atom") | .keys')
  source=$(printf '%s' "$plan" | jq -r \
    '.actions[] | select(.name == "ship/atom" and .decision == "reuse" and .reuse.type == "workload") | .reuse.source')
fi
if [ -n "$source" ]; then
  install_atom "$source"
  printf 'reused exact Plumb atom %s for %s\n' "$PLUMB_BUILD_COMMIT" "$host"
else
  PLUMB_BUILD_SOURCE=1 CARGO_TARGET_DIR="$target" cargo --config "$atom/.cargo/config.toml" build --quiet --locked --manifest-path "$atom/Cargo.toml" --bin plumb
  tar -czf "$archive" -C "$target/debug" plumb
  printf 'built exact Plumb atom %s for %s\n' "$PLUMB_BUILD_COMMIT" "$host"
fi
mkdir -p "$bin"
cp "$target/debug/plumb" "$bin/plumb"
tool="$bin/plumb"
printf '%s\n' "$bin" >> "$GITHUB_PATH"
"$tool" --version
if [ "$mode" = exact ]; then
  PLUMB_HOME="$RUNNER_TEMP/plumb-home-$configuration"
  export PLUMB_HOME
  install_configuration "$PLUMB_HOME/configurations"
  printf 'PLUMB_HOME=%s\n' "$PLUMB_HOME" >> "$GITHUB_ENV"
else
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
    cp "$target/debug/plumb" "$bin/plumb"
    printf 'accepted exact Plumb atom inventory winner %s for %s\n' "$winner" "$host"
  else
    workload_digest=$(sha256sum "$archive" | cut -d' ' -f1)
    source="$inventory_base/workloads/$workload_digest.tgz"
    install_atom "$source"
    cp "$target/debug/plumb" "$bin/plumb"
    printf 'confirmed exact Plumb atom visibility %s for %s\n' "$source" "$host"
  fi
fi
if [ -n "$source" ] && [ -n "${GITHUB_OUTPUT:-}" ]; then
  printf 'source=%s\n' "$source" >> "$GITHUB_OUTPUT"
fi
if [ -n "$source" ] && [ -n "${GITHUB_ENV:-}" ]; then
  printf 'PLUMB_ATOM_SOURCE=%s\n' "$source" >> "$GITHUB_ENV"
fi
