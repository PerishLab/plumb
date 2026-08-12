#!/bin/sh
set -eu

manifest=${1:-plumb.toml}
test -f "$manifest"
if grep -q '^\[\[document\]\]' "$manifest"; then
  printf '%s\n' "$manifest already declares document bindings" >&2
  exit 1
fi

draft=$(mktemp "${manifest}.migration.XXXXXX")
trap 'rm -f "$draft"' EXIT HUP INT TERM
awk '
/^\[\[lock\]\][[:space:]]*$/ { skip = 1; next }
/^\[skill\][[:space:]]*$/ { skip = 1; next }
/^\[/ { skip = 0 }
!skip { print }
' "$manifest" > "$draft"

printf '%s\n' \
  '' \
  '[[document]]' \
  'strategy = "agent"' \
  'source = [{ path = ".", seal = "" }]' \
  'target-seal = ""' >> "$draft"

if test -d skills; then
  for skill in skills/*; do
    test -d "$skill" || continue
    test -f "$skill/SKILL.md"
    test -f "$skill/PATHS.md"
    test -f "$skill/SCENARIOS.md"
    name=${skill##*/}
    printf '%s\n' \
      '' \
      '[[document]]' \
      'strategy = "brief"' \
      "name = \"$name\"" \
      'source = [{ path = ".", seal = "" }]' \
      'target-seal = ""' >> "$draft"
  done
fi

mv "$draft" "$manifest"
trap - EXIT HUP INT TERM
printf '%s\n' "$manifest now carries unaffirmed document bindings"
