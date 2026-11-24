use std::path::Path;
use std::process::Command;

pub(super) fn git(root: &Path, arguments: &[&str]) -> String {
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

pub(super) fn fixture() -> tempfile::TempDir {
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
