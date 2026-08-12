use std::path::Path;
use std::process::Command;

#[test]
fn evidence() {
    let fixture = super::fixture();
    let root = fixture.path();
    std::fs::create_dir_all(root.join("crates/tool/src")).expect("source seat");
    std::fs::write(root.join("crates/tool/src/main.rs"), "fn main() {}\n").expect("source");
    std::fs::write(root.join("AGENTS.md"), "# Agents\n").expect("target");
    declare(root, "", "");
    track(root);

    let bare = super::run(&["doctor", root.to_str().expect("utf8 root")]);
    assert!(
        bare.contains("source crates/tool/src is unaffirmed"),
        "{bare}"
    );
    assert!(bare.contains("target is unaffirmed"), "{bare}");

    let proposal = super::run(&["document", root.to_str().expect("utf8 root")]);
    let source = value(&proposal, "source crates/tool/src seal");
    let target = value(&proposal, "target-seal");
    declare(root, source, target);
    let clean = super::run(&["doctor", root.to_str().expect("utf8 root")]);
    assert!(clean.contains("true to the skeleton"), "{clean}");

    std::fs::write(
        root.join("crates/tool/src/main.rs"),
        "fn main() { panic!() }\n",
    )
    .expect("drift source");
    let source = super::run(&["doctor", root.to_str().expect("utf8 root")]);
    assert!(
        source.contains("invalidated by source crates/tool/src"),
        "{source}"
    );

    std::fs::write(root.join("crates/tool/src/main.rs"), "fn main() {}\n").expect("restore source");
    std::fs::write(root.join("AGENTS.md"), "# Agents\n\nChanged.\n").expect("drift target");
    let target = super::run(&["doctor", root.to_str().expect("utf8 root")]);
    assert!(target.contains("binding topology changed"), "{target}");
}

#[test]
fn admission() {
    let fixture = super::fixture();
    let root = fixture.path();
    std::fs::create_dir_all(root.join("crates/tool/src")).expect("source seat");
    std::fs::write(root.join("crates/tool/src/main.rs"), "fn main() {}\n").expect("source");
    std::fs::write(root.join("AGENTS.md"), "# Agents\n").expect("target");
    std::fs::write(root.join("README.md"), "# Readme\n").expect("foreign prose");
    declare(root, "", "");
    track(root);
    let out = super::run(&["doctor", root.to_str().expect("utf8 root")]);
    assert!(
        out.contains("outside the closed document surface: README.md"),
        "{out}"
    );
}

#[test]
fn closed() {
    let fixture = super::fixture();
    let root = fixture.path();
    std::fs::write(
        root.join("plumb.toml"),
        "[skill]\nstrategy = \"brief\"\n\n[[document]]\nstrategy = \"agent\"\nsource = [{ path = \"crates/tool/src\" }]\n",
    )
    .expect("manifest");
    let out = super::run(&["doctor", root.to_str().expect("utf8 root")]);
    assert!(out.contains("cannot coexist with [skill]"), "{out}");
}

#[test]
fn magnitude() {
    let fixture = super::fixture();
    let root = fixture.path();
    std::fs::create_dir_all(root.join("crates/tool/src")).expect("source seat");
    std::fs::write(root.join("crates/tool/src/main.rs"), "fn main() {}\n").expect("source");
    std::fs::write(root.join("AGENTS.md"), "line\n".repeat(321)).expect("target");
    declare(root, "", "");
    track(root);
    let out = super::run(&["doctor", root.to_str().expect("utf8 root")]);
    assert!(
        out.contains("AGENTS.md has 321 Markdown lines, above agent budget 320"),
        "{out}"
    );
}

#[test]
fn inline() {
    let fixture = super::fixture();
    let root = fixture.path();
    std::fs::write(root.join("AGENTS.md"), "# Agents\n").expect("target");
    std::fs::write(
        root.join("plumb.toml"),
        "[[document]]\nstrategy = \"agent\"\nsource = [{ path = \"plumb.toml\", seal = \"\" }]\ntarget-seal = \"\"\n",
    )
    .expect("manifest");
    track(root);
    let proposal = super::run(&["document", root.to_str().expect("utf8 root")]);
    let source = value(&proposal, "source plumb.toml seal");
    let target = value(&proposal, "target-seal");
    std::fs::write(
        root.join("plumb.toml"),
        format!(
            "[[document]]\nstrategy = \"agent\"\nsource = [{{ path = \"plumb.toml\", seal = \"{source}\" }}]\ntarget-seal = \"{target}\"\n"
        ),
    )
    .expect("affirm manifest");
    let out = super::run(&["doctor", root.to_str().expect("utf8 root")]);
    assert!(out.contains("true to the skeleton"), "{out}");
}

fn declare(root: &Path, source: &str, target: &str) {
    std::fs::write(
        root.join("plumb.toml"),
        format!(
            "[[document]]\nstrategy = \"agent\"\nsource = [{{ path = \"crates/tool/src\", seal = \"{source}\" }}]\ntarget-seal = \"{target}\"\n"
        ),
    )
    .expect("manifest");
}

fn track(root: &Path) {
    let status = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["add", "."])
        .status()
        .expect("git add");
    assert!(status.success());
}

fn value<'a>(shown: &'a str, key: &str) -> &'a str {
    shown
        .lines()
        .find(|line| line.contains(key))
        .and_then(|line| line.split('"').nth(1))
        .expect("proposal value")
}
