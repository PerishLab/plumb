#!/usr/bin/env bash
set -euo pipefail

for name in PLUMB_RELEASES_PUBLIC_URL RELEASE_CHANNEL RELEASE_VERSION R2_METADATA_URL RUNNER_TEMP; do
  [ -n "${!name:-}" ] || { echo "$name is required" >&2; exit 1; }
done

metadata="$RUNNER_TEMP/plumb-release-metadata.json"
curl -fsSL "$R2_METADATA_URL?run=${CI_RUN_ID:-local}" -o "$metadata"
public="${PLUMB_RELEASES_PUBLIC_URL%/}"
jq -e \
  --arg channel "$RELEASE_CHANNEL" \
  --arg version "$RELEASE_VERSION" \
  --arg unix "$public/manage.sh" \
  --arg windows "$public/manage.ps1" \
  '
  (.channel == $channel)
  and (.releaseVersion == $version)
  and (.manage.unix == $unix)
  and (.manage.windows == $windows)
  and (if .channel == "beta"
        then (.betaVersion == $version)
          and (("v" + .baseVersion + "-beta." + (.betaNumber | tostring)) == $version)
        else (.stableVersion == $version) end)
  and (.artifacts | to_entries | all(.value.url | type == "string" and length > 0))
  ' "$metadata" >/dev/null

for url in $(jq -r '(.artifacts[].url), .manage.unix, .manage.windows' "$metadata"); do
  curl -fsSI "$url" >/dev/null
done
echo "R2 release verified: $RELEASE_VERSION"
