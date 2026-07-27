#!/usr/bin/env bash
set -euo pipefail

summary="${GITHUB_STEP_SUMMARY:-/dev/stdout}"
{
  echo "## Plumb ${RELEASE_CHANNEL} release"
  echo
  echo "- version: ${RELEASE_VERSION}"
  echo "- state: ${STATE_SOURCE:-workflow input}"
  echo "- prefix: ${R2_VERSION_PREFIX}"
  echo "- metadata: ${R2_METADATA_URL}"
  echo "- version metadata: ${R2_VERSION_METADATA_URL}"
  echo "- Unix manager: ${PLUMB_RELEASES_PUBLIC_URL%/}/manage.sh"
  echo "- Windows manager: ${PLUMB_RELEASES_PUBLIC_URL%/}/manage.ps1"
} >> "$summary"
