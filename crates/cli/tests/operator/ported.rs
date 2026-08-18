use super::datum::lined;
use super::forgejo::{Court, serve};
use super::stable::{command, run};
use std::process::Command;

#[test]
fn late() {
    let fixture = tempfile::tempdir().expect("fixture");
    let bare = tempfile::tempdir().expect("bare");
    let root = fixture.path();
    let cut = root.join("cut");
    let (url, _) = serve(Court::Prepare(true, cut.clone()), 5);
    let origin = format!("{url}/test/probe.git");
    let head = lined(root, &origin, bare.path(), "release/v1.3.0");
    std::fs::write(&cut, &head).expect("cut");

    std::fs::write(root.join("stray"), "released, never ported\n").expect("stray");
    run(Command::new("git").args(["add", "-A"]).current_dir(root));
    run(Command::new("git")
        .args([
            "commit",
            "-q",
            "-m",
            "Carry work only the release line holds",
        ])
        .current_dir(root));
    run(Command::new("git")
        .args(["tag", "v1.2.0", "HEAD"])
        .current_dir(root));
    run(Command::new("git")
        .args(["update-ref", "refs/remotes/origin/main", &head])
        .current_dir(root));

    let output = command(root, &["stable", "prepare", "--version", "1.3.0"]);
    assert!(!output.status.success());
    let said = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(said.contains("stable v1.2.0 stands at"), "{said}");
    assert!(
        said.contains("plumb stable packport --version v1.2.0"),
        "{said}"
    );
}
