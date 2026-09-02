mod worker;

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

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
    std::fs::write(root.join("pnpm-lock.yaml"), "lockfileVersion: '9.0'\n").expect("pnpm lock");
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
        &root.join("bin/corepack"),
        "#!/bin/sh\nset -eu\nprintf 'corepack %s\\n' \"$*\" >> \"$SITE_CALLS\"\n",
    );
    file(
        &root.join("bin/curl"),
        r#"#!/bin/sh
set -eu
url=
while [ $# -gt 0 ]; do
  case "$1" in
    --request|--write-out|--config|--header) shift 2 ;;
    --url) url=$2; shift 2 ;;
    --silent|--show-error|--location) shift ;;
    *) url=$1; shift ;;
  esac
done
cat >/dev/null
printf 'GET %s\n' "$url" >> "$SITE_CALLS"
case "$url" in
  */workers/scripts/probe/subdomain) printf '{"success":true,"result":{"previews_enabled":true}}' ;;
  */workers/subdomain) printf '{"success":true,"result":{"subdomain":"probeaccount"}}' ;;
  https://abcdefgh-probe.probeaccount.workers.dev) printf '200' ;;
  https://site.test/) printf '200' ;;
  https://workflow.example/worker.tgz) cat "$FAKE_WORKER_WORKLOAD" ;;
  *) printf '{"message":"unexpected"}\n500' ;;
esac
"#,
    );
    file(
        &root.join("bin/aws"),
        "#!/bin/sh\ncase \"$*\" in *get-object*) echo NoSuchKey >&2; exit 1;; *) echo '{}';; esac\n",
    );
}
