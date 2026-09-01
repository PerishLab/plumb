use std::path::Path;
use std::process::{Command, Output};

pub struct Fixture<'a> {
    pub root: &'a Path,
    pub tools: &'a Path,
}

pub fn run(command: &mut Command) -> Output {
    let output = command.output().expect("plumb should run");
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

impl Fixture<'_> {
    pub fn command(&self) -> Command {
        let mut held = Command::new(env!("CARGO_BIN_EXE_plumb"));
        let path = format!(
            "{}:{}",
            self.tools.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        held.env("PATH", path)
            .env("FAKE_S3_ROOT", self.root)
            .env("PLUMB_HOME", self.root)
            .env("PLUMB_RELEASE_ROOT", self.root);
        held
    }

    pub fn archive(&self, artifacts: &Path, version: &str) {
        use std::os::unix::fs::PermissionsExt;

        let seat = self.root.join("binary");
        std::fs::create_dir_all(&seat).expect("binary root");
        let binary = seat.join("probe");
        std::fs::write(&binary, format!("#!/bin/sh\nprintf 'probe {version}\\n'\n"))
            .expect("probe binary");
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755))
            .expect("binary mode");
        run(Command::new("tar").args([
            "-C",
            seat.to_str().expect("binary root path"),
            "-czf",
            artifacts
                .join("probe-x86_64-unknown-linux-gnu.tar.gz")
                .to_str()
                .expect("artifact path"),
            "probe",
        ]));
    }

    pub fn seed(&self) {
        use std::os::unix::fs::PermissionsExt;

        super::support::stock(&self.root.join("configurations"), &[]);
        std::fs::write(self.root.join("plumb.toml"), SPEC).expect("release manifest");
        run(Command::new("git")
            .arg("-C")
            .arg(self.root)
            .args(["init", "-q"]));
        run(Command::new("git")
            .arg("-C")
            .arg(self.root)
            .args(["config", "user.name", "Fixture"]));
        run(Command::new("git").arg("-C").arg(self.root).args([
            "config",
            "user.email",
            "fixture@example.test",
        ]));
        for (name, text) in [("aws", AWS), ("curl", CURL)] {
            let path = self.tools.join(name);
            std::fs::write(&path, text).expect("fake tool");
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
                .expect("tool mode");
        }
    }

    pub fn candidate(&self) -> String {
        run(Command::new("git")
            .arg("-C")
            .arg(self.root)
            .args(["add", "plumb.toml"]));
        run(Command::new("git")
            .arg("-C")
            .arg(self.root)
            .args(["commit", "-qm", "candidate"]));
        String::from_utf8(
            run(Command::new("git")
                .arg("-C")
                .arg(self.root)
                .args(["rev-parse", "HEAD"]))
            .stdout,
        )
        .expect("candidate utf8")
        .trim()
        .to_string()
    }

    pub fn origin(&self, bare: &Path) {
        run(Command::new("git").args(["init", "-q", "--bare"]).arg(bare));
        run(Command::new("git").arg("-C").arg(self.root).args([
            "remote",
            "add",
            "origin",
            &format!("file://{}", bare.display()),
        ]));
        std::fs::create_dir_all(self.root.join(".plumb/releases/v1.2.0")).expect("datum root");
        std::fs::write(
            self.root.join(".plumb/releases/v1.2.0/datum.toml"),
            "schema = 1\nversion = \"v1.2.0\"\n",
        )
        .expect("datum");
        run(Command::new("git")
            .arg("-C")
            .arg(self.root)
            .args(["add", ".plumb"]));
    }

    pub fn line(&self, commit: &str) {
        for reference in ["HEAD:refs/heads/main", "HEAD:refs/heads/release/v1.2.0"] {
            run(Command::new("git")
                .arg("-C")
                .arg(self.root)
                .args(["push", "-q", "origin", reference]));
        }
        let actual = run(Command::new("git")
            .arg("-C")
            .arg(self.root)
            .args(["rev-parse", "HEAD"]));
        assert_eq!(String::from_utf8_lossy(&actual.stdout).trim(), commit);
    }

    pub fn marker(&self, version: &str) {
        run(Command::new("git").arg("-C").arg(self.root).args([
            "tag",
            "-a",
            version,
            "-m",
            &format!("probe {version}"),
        ]));
        run(Command::new("git").arg("-C").arg(self.root).args([
            "push",
            "-q",
            "origin",
            &format!("refs/tags/{version}"),
        ]));
    }

    pub fn track(&self, path: &str) {
        run(Command::new("git")
            .arg("-C")
            .arg(self.root)
            .args(["add", path]));
    }

    pub fn tag(&self, version: &str) {
        run(Command::new("git")
            .arg("-C")
            .arg(self.root)
            .args(["tag", version]));
    }

    pub fn seal(&self, version: &str) {
        let seat = self.root.join("releases/v1/releases/beta").join(version);
        std::fs::create_dir_all(&seat).expect("published seal root");
        let body = serde_json::json!({
            "schema": 1,
            "product": "plumb",
            "channel": "beta",
            "releaseVersion": version,
            "commit": "0000000000000000000000000000000000000000",
            "url": format!("https://releases.test/v1/releases/beta/{version}/seal.json"),
            "generator": { "version": version, "template": "0" },
            "artifacts": {},
            "managers": {},
        });
        std::fs::write(
            seat.join("seal.json"),
            format!("{}\n", serde_json::to_string_pretty(&body).expect("seal")),
        )
        .expect("published seal");
    }
}

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
    if [ "${FAKE_S3_GET_FAILURE:-}" = true ] && [ ! -f "$FAKE_S3_ROOT/get-failed" ]; then
      touch "$FAKE_S3_ROOT/get-failed"
      echo "Connection broken: IncompleteRead" >&2
      exit 1
    fi
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
status=false
while [ $# -gt 0 ]; do
  case "$1" in
    --output|-o) output=$2; shift 2 ;;
    --retry|--retry-delay) shift 2 ;;
    --write-out) status=true; shift 2 ;;
    --fail|--silent|--show-error|--location|--retry-all-errors|--head) shift ;;
    *) url=$1; shift ;;
  esac
done
case "$url" in
  https://releases.test/*) path="$FAKE_S3_ROOT/releases/${url#https://releases.test/}" ;;
  https://depot.test/*) path="$FAKE_S3_ROOT/depot/${url#https://depot.test/}" ;;
  *) exit 1 ;;
esac
if [ -f "$path" ]; then
  if [ -n "$output" ]; then cp "$path" "$output"; else cat "$path"; fi
  [ "$status" = false ] || printf '200'
elif [ "$status" = true ]; then
  printf '404'
else
  exit 1
fi
"#;
