pub const SPEC: &str = r#"
[release]
product = "probe"
authority = "https://releases.test"
binaries = ["probe"]
targets = ["x86_64-unknown-linux-gnu"]
"#;

pub const AWS: &str = r#"#!/bin/sh
set -eu
while [ "$1" != s3api ]; do shift; done
shift
operation=$1
shift
bucket=
key=
body=
match=
absent=false
destination=
while [ $# -gt 0 ]; do
  case "$1" in
    --bucket) bucket=$2; shift 2 ;;
    --key) key=$2; shift 2 ;;
    --body) body=$2; shift 2 ;;
    --if-none-match) absent=true; shift 2 ;;
    --if-match) match=$2; shift 2 ;;
    --content-type|--cache-control|--output) shift 2 ;;
    --no-cli-pager) shift ;;
    *) destination=$1; shift ;;
  esac
done
path="$FAKE_S3_ROOT/$bucket/$key"
etag() {
  sha256sum "$1" | awk '{print $1}'
}
case "$operation" in
  put-object)
    if [ "$absent" = true ] && [ -f "$path" ]; then
      echo "PreconditionFailed 412" >&2
      exit 1
    fi
    if [ -n "$match" ]; then
      [ -f "$path" ] && [ "$(etag "$path")" = "$match" ] || {
        echo "PreconditionFailed 412" >&2
        exit 1
      }
    fi
    mkdir -p "$(dirname "$path")"
    cp "$body" "$path"
    ;;
  head-object)
    [ -f "$path" ] || {
      echo "NoSuchKey 404" >&2
      exit 1
    }
    printf '{"ETag":"%s"}\n' "$(etag "$path")"
    ;;
  get-object)
    [ -f "$path" ] || {
      echo "NoSuchKey 404" >&2
      exit 1
    }
    cp "$path" "$destination"
    printf '{}\n'
    ;;
  *) echo "unsupported fake aws operation: $operation" >&2; exit 1 ;;
esac
"#;

pub const CURL: &str = r#"#!/bin/sh
set -eu
output=
url=
while [ $# -gt 0 ]; do
  case "$1" in
    --output|-o) output=$2; shift 2 ;;
    --retry|--retry-delay) shift 2 ;;
    --fail|--silent|--show-error|--location|--retry-all-errors) shift ;;
    *) url=$1; shift ;;
  esac
done
key=${url#https://releases.test/}
cp "$FAKE_S3_ROOT/releases/$key" "$output"
"#;
