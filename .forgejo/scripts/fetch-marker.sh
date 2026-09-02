#!/bin/bash
set -euo pipefail

marker=$1
reference="refs/tags/$marker"
remote=${2:-origin}
references=("+$reference:$reference")
case "$marker" in
  *-*) ;;
  *) references+=("+refs/tags/$marker-*:refs/tags/$marker-*") ;;
esac
fetch=(git)
if [ -n "${PLUMB_CHECKOUT_TOKEN:-}" ]; then
  basic=$(printf 'x-access-token:%s' "$PLUMB_CHECKOUT_TOKEN" | base64 | tr -d '\n')
  fetch+=(-c "http.extraheader=AUTHORIZATION: basic $basic")
fi
for attempt in 1 2 3; do
  if timeout --kill-after=5s 45s \
    "${fetch[@]}" fetch --no-tags --depth=1 "$remote" "${references[@]}"; then
    git rev-parse --verify "$reference^{commit}" >/dev/null
    exit 0
  fi
  sleep "$attempt"
done
printf 'cannot fetch release marker %s\n' "$marker" >&2
exit 1
