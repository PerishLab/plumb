#!/usr/bin/env bash
set -euo pipefail

for name in PLUMB_RELEASES_S3_AK PLUMB_RELEASES_S3_SK PLUMB_RELEASES_S3_BUCKET PLUMB_RELEASES_S3_URL PLUMB_RELEASES_PUBLIC_URL RELEASE_CHANNEL R2_ACCESS_PROBE_NAME; do
  if [ -z "${!name:-}" ]; then
    echo "$name is required" >&2
    exit 1
  fi
done

temp=${RUNNER_TEMP:-.local/tmp}
mkdir -p "$temp"
probe="$temp/plumb-r2-access.txt"
key="$RELEASE_CHANNEL/.ci-access-check/$R2_ACCESS_PROBE_NAME.txt"
printf 'run=%s\nsha=%s\nchannel=%s\n' \
  "${CI_RUN_ID:-local}" "${CI_COMMIT:-unknown}" "$RELEASE_CHANNEL" > "$probe"

AWS_ACCESS_KEY_ID="$PLUMB_RELEASES_S3_AK" \
AWS_SECRET_ACCESS_KEY="$PLUMB_RELEASES_S3_SK" \
AWS_DEFAULT_REGION=auto \
AWS_EC2_METADATA_DISABLED=true \
aws --endpoint-url "${PLUMB_RELEASES_S3_URL%/}" s3api put-object \
  --bucket "$PLUMB_RELEASES_S3_BUCKET" \
  --key "$key" \
  --body "$probe" \
  --content-type "text/plain; charset=utf-8" \
  --cache-control "no-store" \
  --no-cli-pager >/dev/null

echo "R2 access ok: $key"
