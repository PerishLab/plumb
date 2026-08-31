use super::world::{Court, serve};
use sha2::{Digest, Sha256};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

const COMMIT: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn file(path: &Path, text: &str) {
    std::fs::write(path, text).expect("file");
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).expect("mode");
}

fn digest(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

pub(super) fn seed(root: &Path) {
    let bin = root.join("bin");
    std::fs::create_dir(&bin).expect("bin");
    std::fs::write(
        root.join("plumb.toml"),
        r#"[release]
product = "probe"
authority = "https://releases.test"
binaries = ["probe"]
targets = ["x86_64-unknown-linux-gnu"]
"#,
    )
    .expect("manifest");
    let seal = format!(
        r#"{{"schema":1,"product":"probe","channel":"stable","releaseVersion":"v1.2.0","commit":"{COMMIT}","url":"https://releases.test/seal.json","generator":{{"version":"v0.18.8","template":"test"}},"artifacts":{{}},"managers":{{}}}}"#
    );
    std::fs::write(root.join("seal.json"), &seal).expect("seal");
    let pointer = format!(
        r#"{{"schema":1,"product":"probe","channel":"stable","releaseVersion":"v1.2.0","commit":"{COMMIT}","seal":{{"name":"seal.json","mime":"application/json","sha256":"{}","size":{},"url":"https://releases.test/seal.json"}},"managers":{{}}}}"#,
        digest(&seal),
        seal.len()
    );
    std::fs::write(root.join("pointer.json"), pointer).expect("pointer");
    file(
        &bin.join("git"),
        r#"#!/bin/sh
set -eu
printf 'git %s\n' "$*" >> "$COURT_CALLS"
case "$*" in
  "rev-parse --show-toplevel") printf '%s\n' "$COURT_ROOT" ;;
  "remote get-url origin") printf '%s\n' "https://forge.test/test/probe.git" ;;
  "fetch --prune origin") ;;
  "fetch origin refs/heads/rejoin/v1.2.0") ;;
  "rev-parse FETCH_HEAD^{commit}") printf '%s\n' "dddddddddddddddddddddddddddddddddddddddd" ;;
  "rev-parse origin/main^{commit}") printf '%s\n' "cccccccccccccccccccccccccccccccccccccccc" ;;
  "rev-parse origin/main^{tree}") printf '%s\n' "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee" ;;
  "rev-parse cccccccccccccccccccccccccccccccccccccccc^{tree}") printf '%s\n' "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee" ;;
  "rev-parse aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa^{tree}") printf '%s\n' "ffffffffffffffffffffffffffffffffffffffff" ;;
  "rev-parse dddddddddddddddddddddddddddddddddddddddd^{tree}")
    if [ -f "${COURT_STALE:-}" ]; then
      printf '%s\n' "ffffffffffffffffffffffffffffffffffffffff"
    else
      printf '%s\n' "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
    fi
    ;;
  "rev-list --parents --max-count=1 dddddddddddddddddddddddddddddddddddddddd")
    if [ -f "${COURT_STALE:-}" ]; then
      printf '%s\n' "dddddddddddddddddddddddddddddddddddddddd aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
    else
      printf '%s\n' "dddddddddddddddddddddddddddddddddddddddd cccccccccccccccccccccccccccccccccccccccc bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
    fi
    ;;
  "commit-tree "*) printf '%s\n' "dddddddddddddddddddddddddddddddddddddddd" ;;
  "diff-tree --quiet "*) ;;
  "push --force-with-lease origin "*) ;;
  "merge-base --is-ancestor aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa origin/main")
    test ! -f "${COURT_DIVERGED:-}"
    ;;
  "merge-base --is-ancestor "*) test -f "$COURT_SETTLED" ;;
  *) printf '%s\n' "unexpected git: $*" >&2; exit 1 ;;
esac
"#,
    );
    file(
        &bin.join("curl"),
        r#"#!/bin/sh
set -eu
method=GET
output=
body=
url=
while [ $# -gt 0 ]; do
  case "$1" in
    --request) method=$2; shift 2 ;;
    --output) output=$2; shift 2 ;;
    --data-binary) body=$2; shift 2 ;;
    --write-out|--header) shift 2 ;;
    --config) shift 2 ;;
    --fail|--silent|--show-error|--location) shift ;;
    --retry) shift 2 ;;
    *) url=$1; shift ;;
  esac
done
cat >/dev/null
printf '%s %s %s\n' "$method" "$url" "$body" >> "$COURT_CALLS"
if [ -n "$output" ]; then
  case "$url" in
    */stable.json) cp "$COURT_ROOT/pointer.json" "$output" ;;
    */seal.json) cp "$COURT_ROOT/seal.json" "$output" ;;
    *) exit 1 ;;
  esac
  exit 0
fi
case "$url" in
  */stable.json) cat "$COURT_ROOT/pointer.json"; printf '\n200' ;;
  */api/v1/user) printf '{"login":"operator"}\n200' ;;
  */branch_protections/*) printf '{"message":"The target couldn'\''t be found."}\n404' ;;
  */branch_protections)
    printf '%s' "$body" | sed 's/^{/{"branch_name":"release\/v1.2.0",/'
    printf '\n201'
    ;;
  */branches/*) printf '{"commit":{"id":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}}\n200' ;;
  */pulls\?*) printf '[]\n200' ;;
  */pulls) printf '{"number":12}\n201' ;;
  */commits/*/status)
    printf '{"statuses":[{"context":"guard / guard (pull_request)","status":"success","updated_at":"2026-01-01T00:00:00Z"}]}\n200'
    ;;
  */pulls/12/merge) touch "$COURT_SETTLED"; printf '\n204' ;;
  *) printf '{"message":"unexpected"}\n500' ;;
esac
"#,
    );
}

#[test]
fn retains() {
    let fixture = tempfile::tempdir().expect("fixture");
    let settled = fixture.path().join("settled");
    let (forge, forge_calls) = serve(Court::Rejoin(settled.clone()), 8);
    seed(fixture.path());
    let calls = fixture.path().join("calls");
    let path = format!(
        "{}:{}",
        fixture.path().join("bin").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["version", "rejoin", "--version", "v1.2.0"])
        .current_dir(fixture.path())
        .env("PATH", path)
        .env("FORGEJO_TOKEN", "test-token")
        .env("FORGEJO_URL", forge)
        .env("COURT_ROOT", fixture.path())
        .env("COURT_CALLS", &calls)
        .env("COURT_SETTLED", &settled)
        .output()
        .expect("plumb");
    assert!(
        output.status.success(),
        "{} {:?}",
        String::from_utf8_lossy(&output.stderr),
        forge_calls.lock().expect("forge calls")
    );
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("retained frozen release/v1.2.0"), "{text}");
    let calls = std::fs::read_to_string(calls).expect("calls");
    assert!(!calls.contains("DELETE"), "{calls}");
    assert!(
        calls.contains(
            "git commit-tree eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee -p cccccccccccccccccccccccccccccccccccccccc -p bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        ),
        "the rejoin commit must keep main's tree and name the release as its second parent: {calls}"
    );
    assert!(
        calls.contains(
            "git push --force-with-lease origin dddddddddddddddddddddddddddddddddddddddd:refs/heads/rejoin/v1.2.0"
        ),
        "the guarded pull must stand on the topology-only commit: {calls}"
    );
    let held = forge_calls.lock().expect("forge calls");
    let merge = held
        .iter()
        .find(|line| line.contains("/pulls/12/merge"))
        .expect("merge");
    assert!(merge.contains(r#""delete_branch_after_merge":false"#));
    assert!(merge.contains(r#""Do":"fast-forward-only""#), "{merge}");
}

#[test]
fn resumes() {
    let fixture = tempfile::tempdir().expect("fixture");
    let settled = fixture.path().join("settled");
    let resume = fixture.path().join("resume");
    std::fs::write(&resume, "open pull already proved\n").expect("resume marker");
    let (forge, held) = serve(Court::Rejoin(settled.clone()), 10);
    seed(fixture.path());
    let calls = fixture.path().join("calls");
    let path = format!(
        "{}:{}",
        fixture.path().join("bin").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["version", "rejoin", "--version", "v1.2.0"])
        .current_dir(fixture.path())
        .env("PATH", path)
        .env("FORGEJO_TOKEN", "test-token")
        .env("FORGEJO_URL", forge)
        .env("COURT_ROOT", fixture.path())
        .env("COURT_CALLS", &calls)
        .env("COURT_SETTLED", &settled)
        .output()
        .expect("plumb");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let calls = std::fs::read_to_string(calls).expect("calls");
    assert!(
        calls.contains("git fetch origin refs/heads/rejoin/v1.2.0"),
        "{calls}"
    );
    assert!(!calls.contains("git commit-tree"), "{calls}");
    assert!(!calls.contains("git push"), "{calls}");
    let held = held.lock().expect("forge calls");
    assert!(
        !held
            .iter()
            .any(|call| call.starts_with("POST ") && call.contains("/pulls HTTP/1.1")),
        "{held:?}"
    );
    assert!(
        held.iter().any(|call| call.contains("/pulls/12/merge")),
        "{held:?}"
    );
}
