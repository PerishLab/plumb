pub(crate) const CLIENT: &str = r#"#!/bin/sh
set -eu
scenario=$(cat "$PLUMB_TEST_SCENARIO")
[ -z "${DOCKER_AUTH_CONFIG+x}" ]
printf 'seat %s\n' "$REGCTL_CONFIG" >> "$PLUMB_TEST_CALLS"
printf 'regctl %s\n' "$*" >> "$PLUMB_TEST_CALLS"
case "$1 $2" in
  'config set')
    [ "$3" = --docker-cred=false ]
    [ "$4" = --docker-cert=false ]
    printf '{}\n' > "$REGCTL_CONFIG"
    ;;
  'registry login')
    [ "${REGCTL_CONFIG##*/}" = authenticated ]
    [ "$3" = registry.example ]
    [ "$4" = --user ]; [ "$5" = Example ]; [ "$6" = --pass-stdin ]
    [ "$(cat)" = secret ]
    [ "$scenario" != login ]
    printf 'authenticated\n' > "$REGCTL_CONFIG"
    ;;
  'image import') ;;
  'image config')
    if [ "$scenario" = provenance ]; then payload=unproved; else payload=cccccccccccccccccccccccccccccccccccccccc; fi
    printf '{"config":{"Labels":{"org.opencontainers.image.revision":"%s"}}}\n' "$payload"
    ;;
  'image digest')
    if [ "$scenario" = invalid ]; then printf 'invalid\n'; exit; fi
    case "$3" in
      registry.example/*)
        if [ "$scenario" = drift ]; then printf 'sha256:%064d\n' 2; exit; fi
        ;;
      *readback*)
        if [ "$scenario" = readback ]; then printf 'sha256:%064d\n' 2; exit; fi
        ;;
    esac
    printf 'sha256:%064d\n' 1
    ;;
  'image copy')
    case "$3" in
      ocidir:*) [ "$(cat "$REGCTL_CONFIG")" = authenticated ] ;;
      registry.example/*)
        [ "${REGCTL_CONFIG##*/}" = anonymous ]
        [ "$(cat "$REGCTL_CONFIG")" = '{}' ]
        [ "$5" = --force-recursive ]
        [ "$scenario" != anonymous ]
        ;;
      *) exit 98 ;;
    esac
    ;;
  *) exit 97 ;;
esac
"#;
