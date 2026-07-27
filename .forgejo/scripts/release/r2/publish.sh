#!/usr/bin/env bash
set -euo pipefail

for name in PLUMB_RELEASES_S3_AK PLUMB_RELEASES_S3_SK PLUMB_RELEASES_S3_BUCKET PLUMB_RELEASES_S3_URL PLUMB_RELEASES_PUBLIC_URL RELEASE_CHANNEL RELEASE_VERSION RELEASE_ROOT; do
  if [ -z "${!name:-}" ]; then
    echo "$name is required" >&2
    exit 1
  fi
done

root="$RELEASE_ROOT"
public="${PLUMB_RELEASES_PUBLIC_URL%/}"
version_prefix="$RELEASE_CHANNEL/versions/$RELEASE_VERSION"
latest_prefix="$RELEASE_CHANNEL/latest"
metadata="$root/metadata.json"

upload() {
  local path="$1" key="$2" type="$3" cache="$4"
  [ -f "$path" ] || { echo "missing upload file: $path" >&2; exit 1; }
  AWS_ACCESS_KEY_ID="$PLUMB_RELEASES_S3_AK" \
  AWS_SECRET_ACCESS_KEY="$PLUMB_RELEASES_S3_SK" \
  AWS_DEFAULT_REGION=auto \
  AWS_EC2_METADATA_DISABLED=true \
  aws --endpoint-url "${PLUMB_RELEASES_S3_URL%/}" s3api put-object \
    --bucket "$PLUMB_RELEASES_S3_BUCKET" \
    --key "$key" \
    --body "$path" \
    --content-type "$type" \
    --cache-control "$cache" \
    --no-cli-pager >/dev/null
}

content_type() {
  case "$1" in
    *.tar.gz) echo application/gzip ;;
    *.zip) echo application/zip ;;
    *.json) echo "application/json; charset=utf-8" ;;
    *.txt) echo "text/plain; charset=utf-8" ;;
    *.sh) echo "text/x-shellscript; charset=utf-8" ;;
    *) echo application/octet-stream ;;
  esac
}

for path in "$root"/plumb-*.tar.gz "$root"/plumb-*.zip "$root"/checksums.txt; do
  [ -f "$path" ] || continue
  name=$(basename "$path")
  upload "$path" "$version_prefix/$name" "$(content_type "$name")" \
    "public, max-age=31536000, immutable"
done
upload manage.sh manage.sh "text/x-shellscript; charset=utf-8" "public, max-age=60, must-revalidate"
upload manage.ps1 manage.ps1 "text/plain; charset=utf-8" "public, max-age=60, must-revalidate"

artifact() {
  local name="$1" type="$2" path="$root/$1"
  jq -n \
    --arg contentType "$type" \
    --arg name "$name" \
    --argjson size "$(stat -c %s "$path")" \
    --arg url "$public/$version_prefix/$name" \
    '{contentType: $contentType, name: $name, size: $size, url: $url}'
}

artifacts=$(jq -n \
  --argjson linuxX64 "$(artifact plumb-x86_64-unknown-linux-gnu.tar.gz application/gzip)" \
  --argjson darwinArm64 "$(artifact plumb-aarch64-apple-darwin.tar.gz application/gzip)" \
  --argjson darwinX64 "$(artifact plumb-x86_64-apple-darwin.tar.gz application/gzip)" \
  --argjson windowsX64 "$(artifact plumb-x86_64-pc-windows-msvc.zip application/zip)" \
  --argjson checksums "$(artifact checksums.txt 'text/plain; charset=utf-8')" \
  '{linuxX64: $linuxX64, darwinArm64: $darwinArm64, darwinX64: $darwinX64, windowsX64: $windowsX64, checksums: $checksums}')

held=$(jq -n \
  --arg channel "$RELEASE_CHANNEL" \
  --arg releaseVersion "$RELEASE_VERSION" \
  --arg generatedAt "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --arg repository "${CI_REPOSITORY:-unknown}" \
  --arg commit "${CI_COMMIT:-unknown}" \
  --arg runId "${CI_RUN_ID:-local}" \
  --arg runAttempt "${CI_RUN_ATTEMPT:-1}" \
  --arg workflow "${CI_WORKFLOW:-release}" \
  --arg publicUrl "$public" \
  --arg latestMetadataUrl "$public/$latest_prefix/metadata.json" \
  --arg versionMetadataUrl "$public/$version_prefix/metadata.json" \
  --arg versionPrefix "$version_prefix" \
  --arg latestPrefix "$latest_prefix" \
  --arg unix "$public/manage.sh" \
  --arg windows "$public/manage.ps1" \
  --argjson artifacts "$artifacts" \
  '{
    version: 1,
    channel: $channel,
    releaseVersion: $releaseVersion,
    generatedAt: $generatedAt,
    ci: {repository: $repository, commit: $commit, runId: $runId, runAttempt: $runAttempt, workflow: $workflow},
    r2: {publicUrl: $publicUrl, latestMetadataUrl: $latestMetadataUrl, versionMetadataUrl: $versionMetadataUrl, versionPrefix: $versionPrefix, latestPrefix: $latestPrefix},
    manage: {unix: $unix, windows: $windows},
    artifacts: $artifacts
  }')

source="${STATE_SOURCE:-workflow input}"
if [ "$RELEASE_CHANNEL" = beta ]; then
  [[ "$RELEASE_VERSION" =~ ^v([0-9]+\.[0-9]+\.[0-9]+)-beta\.([1-9][0-9]*)$ ]] || {
    echo "invalid beta version: $RELEASE_VERSION" >&2
    exit 1
  }
  base="${BASE_VERSION:-${BASH_REMATCH[1]}}"
  number="${BETA_NUMBER:-${BASH_REMATCH[2]}}"
  [ "$base" = "${BASH_REMATCH[1]}" ] || {
    echo "beta base mismatch: $base != ${BASH_REMATCH[1]}" >&2
    exit 1
  }
  [ "$number" = "${BASH_REMATCH[2]}" ] || {
    echo "beta number mismatch: $number != ${BASH_REMATCH[2]}" >&2
    exit 1
  }
  held=$(printf '%s' "$held" | jq \
    --arg base "$base" --argjson number "$number" --arg source "$source" \
    '.baseVersion = $base | .betaNumber = $number | .betaVersion = .releaseVersion | .stateSource = $source')
else
  held=$(printf '%s' "$held" | jq --arg source "$source" \
    '.stableVersion = .releaseVersion | .stateSource = $source')
fi
printf '%s\n' "$held" > "$metadata"

upload "$metadata" "$version_prefix/metadata.json" "application/json; charset=utf-8" \
  "public, max-age=31536000, immutable"
upload "$metadata" "$latest_prefix/metadata.json" "application/json; charset=utf-8" \
  "public, max-age=60, must-revalidate"
