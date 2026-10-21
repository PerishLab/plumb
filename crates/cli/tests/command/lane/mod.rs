#[cfg(unix)]
mod dispatch;
mod status;

use std::path::Path;
use std::process::{Command, Output};

fn run(arguments: &[&str]) -> Output {
    let home = super::configuration::support::home();
    Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(arguments)
        .env("PLUMB_HOME", home.path())
        .env("CLOUDFLARE_API_TOKEN", "private-provider-value")
        .output()
        .unwrap()
}

fn git(root: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .args([
            "-c",
            "core.hooksPath=",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "user.name=fixture",
            "-c",
            "user.email=fixture@example.test",
            "-C",
        ])
        .arg(root)
        .args(arguments)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
}

fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let app = root.path().join("apps/review");
    std::fs::create_dir_all(&app).unwrap();
    std::fs::write(
        root.path().join("plumb.toml"),
        r#"
[lane]
repository = "PerishLab/crest"
[lane.app.crest]
path = "apps/review"
package = "crest-review"
[lane.app.crest.mapping]
provider = "cfworker"
access = "public"
account = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
resource = "crest-development"
[lane.app.crest.binding.design]
adaptor = "static"
capabilities = ["inspect", "build", "deploy", "dispose"]
authorities = { authorization = "wharf", publication = "wharf" }
[lane.app.crest.binding.review]
adaptor = "static"
capabilities = ["inspect", "deploy"]
authorities = { authorization = "wharf", publication = "wharf" }
"#,
    )
    .unwrap();
    std::fs::write(
        app.join("package.json"),
        r#"{"name":"crest-review","private":true,"scripts":{"build":"touch SHOULD_NOT_EXIST"}}"#,
    )
    .unwrap();
    std::fs::write(app.join("wrangler.jsonc"), r#"{"name":"crest-development","account_id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","compatibility_date":"2026-10-01","workers_dev":false,"assets":{"directory":"./dist","not_found_handling":"404-page"},"previews":{}}"#).unwrap();
    git(root.path(), &["init", "--quiet"]);
    git(root.path(), &["add", "."]);
    git(root.path(), &["commit", "--quiet", "-m", "fixture"]);
    root
}

#[test]
fn surface() {
    let output = run(&["lane", "--help"]);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    for command in ["status", "inspect", "plan", "apply", "discard", "result"] {
        assert!(text.contains(command), "{text}");
    }
    for command in ["dispose", "build", "publish", "repair"] {
        assert!(!run(&["lane", command, "--help"]).status.success());
    }
    assert!(!run(&["lane", "status", "--json"]).status.success());
    let output = run(&["lane", "inspect", "--help"]);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    for flag in [
        "--repository",
        "--app",
        "--lane",
        "--origin",
        "--authority",
        "--digest",
        "--request",
        "--revision",
    ] {
        assert!(text.contains(flag), "{text}");
    }
}
