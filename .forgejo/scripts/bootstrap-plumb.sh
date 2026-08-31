#!/bin/sh
set -eu

atom=${1:-.plumb-atom}
fetch='curl -fsSL --retry 5 --retry-all-errors --retry-delay 1 --connect-timeout 5 --max-time 30'
manager="$RUNNER_TEMP/manage-plumb.sh"
$fetch -o "$manager" "https://releases.plumb.perish.uk/manage.sh"
held=$($fetch "https://releases.plumb.perish.uk/v1/channels/beta.json" 2>/dev/null \
  | sed -n 's/.*"releaseVersion"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p') || held=
if [ -n "$held" ]; then
  seat="$RUNNER_TEMP/plumb-$held"
  sh "$manager" install \
    --channel beta --version "$held" \
    --install-root "$seat/versions" --bin-dir "$seat/bin"
  bin="$seat/bin"
else
  sh "$manager"
  bin="$HOME/.local/bin"
fi
tool="$bin/plumb"
"$tool" depot sync
cargo build --quiet --locked --manifest-path "$atom/Cargo.toml" --bin plumb
cp "$atom/target/debug/plumb" "$tool"
printf '%s\n' "$bin" >> "$GITHUB_PATH"
"$tool" --version
