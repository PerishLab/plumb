#!/usr/bin/env bash
set -eu

atom=${1:?exact Plumb atom is required}
mode=${2:-workload}
graph=$(plumb ship resolve --marker "$PLUMB_RELEASE_MARKER" --atom "$atom")
source=${PLUMB_ATOM_SOURCE:-}
digest=$(printf '%s' "$source" | sed -n 's#^.*/workloads/\([0-9a-fA-F]\{64\}\)\.tgz$#\1#p')
test -n "$digest"
if [ "$mode" = ready ]; then
  test "$(printf '%s' "$graph" | jq -r '.publication_ready')" = true
fi
{
  echo "atom_0=$(printf '%s' "$digest" | cut -c 1-16)"
  echo "atom_1=$(printf '%s' "$digest" | cut -c 17-32)"
  echo "atom_2=$(printf '%s' "$digest" | cut -c 33-48)"
  echo "atom_3=$(printf '%s' "$digest" | cut -c 49-64)"
  echo "channel=$(printf '%s' "$graph" | jq -r '.channel')"
  echo "commit=$(printf '%s' "$graph" | jq -r '.commit')"
  echo "version=$(printf '%s' "$graph" | jq -r '.version')"
  echo "workload=$(printf '%s' "$graph" | jq -c '.workload')"
  echo "workload_missing=$(printf '%s' "$graph" | jq -r '.workload_missing')"
  echo "publication=$(printf '%s' "$graph" | jq -c '.publication')"
  echo "publication_missing=$(printf '%s' "$graph" | jq -r '.publication_missing')"
  echo "publication_ready=$(printf '%s' "$graph" | jq -r '.publication_ready')"
} >> "$GITHUB_OUTPUT"
