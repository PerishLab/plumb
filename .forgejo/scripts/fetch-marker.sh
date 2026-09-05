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

bounded_fetch() {
  "${fetch[@]}" fetch --no-tags --depth=1 "$remote" "${references[@]}" &
  fetch_pid=$!
  (
    sleep 45
    kill -TERM "$fetch_pid" 2>/dev/null || exit 0
    sleep 5
    kill -KILL "$fetch_pid" 2>/dev/null || true
  ) &
  watchdog_pid=$!

  status=0
  wait "$fetch_pid" || status=$?
  kill "$watchdog_pid" 2>/dev/null || true
  wait "$watchdog_pid" 2>/dev/null || true
  return "$status"
}

for attempt in 1 2 3; do
  if bounded_fetch; then
    git rev-parse --verify "$reference^{commit}" >/dev/null
    exit 0
  fi
  sleep "$attempt"
done
printf 'cannot fetch release marker %s\n' "$marker" >&2
exit 1
