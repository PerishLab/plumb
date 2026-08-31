mod server;
mod worker;

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};

fn file(path: &Path, text: &str) {
    std::fs::write(path, text).expect("file");
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).expect("mode");
}

fn seed(root: &Path) {
    for path in ["apps/web/dist", "bin"] {
        std::fs::create_dir_all(root.join(path)).expect("directory");
    }
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\n[workspace.package]\nversion = \"1.2.3\"\n",
    )
    .expect("manifest");
    std::fs::write(
        root.join("apps/web/package.json"),
        r#"{"name":"@probe/web"}"#,
    )
    .expect("package");
    std::fs::write(
        root.join("apps/web/wrangler.jsonc"),
        r#"{"name":"probe","assets":{"directory":"./dist"}}"#,
    )
    .expect("config");
    std::fs::write(
        root.join("apps/web/dist/index.html"),
        r#"<script src="/assets/index-mark.js"></script>"#,
    )
    .expect("index");
    std::fs::write(
        root.join("apps/web/dist/sitemap.xml"),
        "<loc>https://site.test/</loc><loc>https://site.test/guide/</loc>",
    )
    .expect("atlas");
    file(
        &root.join("bin/git"),
        "#!/bin/sh\nset -eu\nprintf '%s\\n' abc123\n",
    );
    file(
        &root.join("bin/pnpm"),
        r#"#!/bin/sh
set -eu
printf '%s %s\n' "$PWD" "$*" >> "$SITE_CALLS"
case "$*" in
  *"wrangler versions upload"*) printf '%s\n' 'Worker Version ID: abcdefgh12345678' ;;
esac
"#,
    );
    file(
        &root.join("bin/curl"),
        r#"#!/bin/sh
set -eu
url=
while [ $# -gt 0 ]; do
  case "$1" in
    --request|--write-out|--config) shift 2 ;;
    --url) url=$2; shift 2 ;;
    --silent|--show-error) shift ;;
    *) url=$1; shift ;;
  esac
done
cat >/dev/null
printf 'GET %s\n' "$url" >> "$SITE_CALLS"
case "$url" in
  */workers/domains)
    case "$SITE_CASE" in
      unknown) printf '{"success":false}\n403' ;;
      unbound) printf '{"success":true,"result":[]}\n200' ;;
      *) printf '{"success":true,"result":[{"hostname":"site.test"}]}\n200' ;;
    esac
    ;;
  */tokens/verify) printf '{"success":true,"result":{"status":"active"}}\n200' ;;
  */workers/services/*) printf '{"success":true,"result":{"id":"probe"}}\n200' ;;
  */workers/scripts/probe/subdomain) printf '{"success":true,"result":{"previews_enabled":true}}' ;;
  */workers/subdomain) printf '{"success":true,"result":{"subdomain":"probeaccount"}}' ;;
  https://abcdefgh-probe.probeaccount.workers.dev) printf '200' ;;
  https://workflow.example/worker.tgz) cat "$FAKE_WORKER_WORKLOAD" ;;
  https://site.test/*)
    case "$SITE_CASE" in
      live) printf '<script src="/assets/index-mark.js"></script>\n200' ;;
      unknown) exit 7 ;;
      *) printf '<script src="/assets/index-old.js"></script>\n200' ;;
    esac
    ;;
  *) printf '{"message":"unexpected"}\n500' ;;
esac
"#,
    );
    file(
        &root.join("bin/aws"),
        "#!/bin/sh\ncase \"$*\" in *get-object*) echo NoSuchKey >&2; exit 1;; *) echo '{}';; esac\n",
    );
}

fn run(root: &Path, args: &[&str], case: &str, blind: bool) -> Output {
    let calls = root.join("calls");
    let cloud = if args.contains(&"deploy") || args.contains(&"inspect") {
        Some(server::serve(
            case,
            &calls,
            if args.contains(&"inspect") { 3 } else { 1 },
        ))
    } else {
        None
    };
    let path = format!(
        "{}:{}",
        root.join("bin").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let mut command = Command::new(env!("CARGO_BIN_EXE_plumb"));
    command
        .args(args)
        .current_dir(root)
        .env("PATH", path)
        .env("SITE_CALLS", calls)
        .env("SITE_CASE", case)
        .env("PLUMB_SITE_ACCOUNT", "account")
        .env(
            "PLUMB_SITE_API",
            cloud
                .as_ref()
                .map(|(url, _)| url.as_str())
                .unwrap_or("https://cloud.test"),
        )
        .env("PLUMB_SITE_DOMAIN", "site.test")
        .env("PLUMB_SITE_TOKEN", "held-secret")
        .env("PLUMB_SITE_TURNS", "1")
        .env("PLUMB_SITE_DELAY", "0");
    if blind {
        command.env("PLUMB_SITE_BLIND", "true");
    }
    let output = command.output().expect("plumb");
    if let Some((_, handle)) = cloud {
        handle.join().expect("cloud server");
    }
    output
}

#[test]
fn plan() {
    let fixture = tempfile::tempdir().expect("fixture");
    seed(fixture.path());
    let output = run(fixture.path(), &["ship", "site", "plan"], "live", false);
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("pnpm --filter @probe/web build"), "{text}");
    assert!(text.contains("https://site.test/guide"), "{text}");
    let calls = std::fs::read_to_string(fixture.path().join("calls")).expect("calls");
    assert!(calls.contains("wrangler deploy --dry-run --domain site.test"));
}

#[test]
fn bare() {
    let fixture = tempfile::tempdir().expect("fixture");
    seed(fixture.path());
    let path = format!(
        "{}:{}",
        fixture.path().join("bin").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["ship", "site", "plan"])
        .current_dir(fixture.path())
        .env("PATH", path)
        .env("SITE_CALLS", fixture.path().join("calls"))
        .env("SITE_CASE", "live")
        .output()
        .expect("plumb");
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("https://<PLUMB_SITE_DOMAIN>/"), "{text}");
}

#[test]
fn deploy() {
    let fixture = tempfile::tempdir().expect("fixture");
    seed(fixture.path());
    let output = run(fixture.path(), &["ship", "site", "deploy"], "live", false);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    for line in ["deployed  yes", "bound     yes", "reachable yes"] {
        assert!(text.contains(line), "{text}");
    }
    let calls = std::fs::read_to_string(fixture.path().join("calls")).expect("calls");
    assert!(!calls.contains("held-secret"), "{calls}");
}

#[test]
fn node() {
    let fixture = tempfile::tempdir().expect("fixture");
    seed(fixture.path());
    std::fs::remove_file(fixture.path().join("Cargo.toml")).expect("remove manifest");
    let output = run(fixture.path(), &["ship", "site", "deploy"], "live", false);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn unproven() {
    let fixture = tempfile::tempdir().expect("fixture");
    seed(fixture.path());
    let output = run(fixture.path(), &["ship", "site", "deploy"], "unknown", true);
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        error.contains("binding is unknown and readback failed"),
        "{error}"
    );
}

#[test]
fn blind() {
    let fixture = tempfile::tempdir().expect("fixture");
    seed(fixture.path());
    let output = run(fixture.path(), &["ship", "site", "deploy"], "stale", true);
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("bound     yes"), "{text}");
    assert!(text.contains("reachable no"), "{text}");
}

#[test]
fn unbound() {
    let fixture = tempfile::tempdir().expect("fixture");
    seed(fixture.path());
    let output = run(
        fixture.path(),
        &["ship", "site", "deploy"],
        "unbound",
        false,
    );
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("is not attached to the worker"), "{error}");
}

#[test]
fn inspect() {
    let fixture = tempfile::tempdir().expect("fixture");
    seed(fixture.path());
    let output = run(fixture.path(), &["ship", "site", "inspect"], "live", false);
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("token: active"), "{text}");
    assert!(text.contains("worker: probe present"), "{text}");
    assert!(text.contains("bound: yes"), "{text}");
}

#[test]
fn insecure() {
    let fixture = tempfile::tempdir().expect("fixture");
    seed(fixture.path());
    let path = format!(
        "{}:{}",
        fixture.path().join("bin").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["ship", "site", "inspect"])
        .current_dir(fixture.path())
        .env("PATH", path)
        .env("PLUMB_SITE_API", "http://cloud.test")
        .output()
        .expect("plumb");
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("PLUMB_SITE_API must use https"), "{error}");
}
