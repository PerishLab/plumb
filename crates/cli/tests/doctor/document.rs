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
    assert!(out.contains("retired document declarations"), "{out}");
}

#[test]
fn required() {
    let fixture = super::fixture();
    let root = fixture.path();
    std::fs::write(root.join("plumb.toml"), "").expect("manifest");
    let out = super::run(&["doctor", root.to_str().expect("utf8 root")]);
    assert!(out.contains("misses document declarations"), "{out}");
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
        out.contains("AGENTS.md has 321 Markdown lines, above agent budget 240"),
        "{out}"
    );

    for leaf in 0..120 {
        std::fs::write(
            root.join(format!("crates/tool/src/leaf{leaf}.rs")),
            "fn held() {}\n",
        )
        .expect("source");
    }
    track(root);
    let wider = super::run(&["doctor", root.to_str().expect("utf8 root")]);
    assert!(
        !wider.contains("above agent budget"),
        "a source that grew must carry a budget that grew: {wider}"
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

#[test]
fn brief() {
    let fixture = super::fixture();
    let root = fixture.path();
    std::fs::create_dir_all(root.join("crates/tool/src")).expect("source seat");
    std::fs::create_dir_all(root.join("skills/tool/references")).expect("extra seat");
    std::fs::write(root.join("crates/tool/src/main.rs"), "fn main() {}\n").expect("source");
    std::fs::write(root.join("AGENTS.md"), "# Agents\n").expect("agent");
    for leaf in ["SKILL.md", "PATHS.md", "SCENARIOS.md"] {
        std::fs::write(root.join("skills/tool").join(leaf), "brief\n").expect("brief");
    }
    std::fs::write(root.join("skills/tool/references/extra.md"), "extra\n").expect("extra");
    std::fs::write(
        root.join("plumb.toml"),
        "[[document]]\nstrategy = \"agent\"\nsource = [{ path = \"crates/tool/src\", seal = \"\" }]\ntarget-seal = \"\"\n\n[[document]]\nstrategy = \"brief\"\nname = \"tool\"\nsource = [{ path = \"crates/tool/src\", seal = \"\" }]\ntarget-seal = \"\"\n",
    )
    .expect("manifest");
    track(root);
    let out = super::run(&["doctor", root.to_str().expect("utf8 root")]);
    assert!(out.contains("skills/tool/references/extra.md"), "{out}");
}

#[test]
fn mass() {
    let fixture = super::fixture();
    let root = fixture.path();
    std::fs::create_dir_all(root.join("crates/tool/src")).expect("source seat");
    std::fs::create_dir_all(root.join("skills/tool")).expect("brief seat");
    std::fs::write(root.join("crates/tool/src/main.rs"), "fn main() {}\n").expect("source");
    std::fs::write(root.join("AGENTS.md"), "# Agents\n").expect("agent");
    std::fs::write(root.join("skills/tool/SKILL.md"), "line\n".repeat(239)).expect("brief");
    std::fs::write(root.join("skills/tool/PATHS.md"), "path\n").expect("paths");
    std::fs::write(root.join("skills/tool/SCENARIOS.md"), "scenario\n").expect("scenarios");
    std::fs::write(
        root.join("plumb.toml"),
        "[[document]]\nstrategy = \"agent\"\nsource = [{ path = \"crates/tool/src\", seal = \"\" }]\ntarget-seal = \"\"\n\n[[document]]\nstrategy = \"brief\"\nname = \"tool\"\nsource = [{ path = \"crates/tool/src\", seal = \"\" }]\ntarget-seal = \"\"\n",
    )
    .expect("manifest");
    track(root);
    let out = super::run(&["doctor", root.to_str().expect("utf8 root")]);
    assert!(
        out.contains("skills/tool has 241 Markdown lines, above brief budget 240"),
        "{out}"
    );
}

#[cfg(unix)]
#[test]
fn regular() {
    let fixture = super::fixture();
    let root = fixture.path();
    std::fs::create_dir_all(root.join("crates/tool/src")).expect("source seat");
    std::fs::create_dir_all(root.join("skills/tool")).expect("brief seat");
    std::fs::write(root.join("crates/tool/src/main.rs"), "fn main() {}\n").expect("source");
    std::fs::write(root.join("AGENTS.md"), "# Agents\n").expect("agent");
    std::fs::write(root.join("skills/tool/SKILL.md"), "brief\n").expect("brief");
    std::fs::write(root.join("skills/tool/SCENARIOS.md"), "scenario\n").expect("scenarios");
    std::os::unix::fs::symlink("SKILL.md", root.join("skills/tool/PATHS.md")).expect("symlink");
    std::fs::write(
        root.join("plumb.toml"),
        "[[document]]\nstrategy = \"agent\"\nsource = [{ path = \"crates/tool/src\", seal = \"\" }]\ntarget-seal = \"\"\n\n[[document]]\nstrategy = \"brief\"\nname = \"tool\"\nsource = [{ path = \"crates/tool/src\", seal = \"\" }]\ntarget-seal = \"\"\n",
    )
    .expect("manifest");
    track(root);
    let out = super::run(&["doctor", root.to_str().expect("utf8 root")]);
    assert!(out.contains("PATHS.md is not a regular file"), "{out}");
}

#[test]
fn command() {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .arg("lock")
        .output()
        .expect("plumb");
    assert!(!output.status.success());
}

#[test]
fn untracked() {
    let fixture = super::fixture();
    let root = fixture.path();
    let seat = root.to_str().expect("utf8 root");
    std::fs::create_dir_all(root.join("crates/tool/src")).expect("source seat");
    std::fs::write(root.join("crates/tool/src/main.rs"), "fn main() {}\n").expect("source");
    std::fs::write(root.join("AGENTS.md"), "# Agents\n").expect("target");
    declare(root, "", "");
    track(root);
    let proposal = super::run(&["document", seat]);
    let source = value(&proposal, "source crates/tool/src seal");
    let target = value(&proposal, "target-seal");
    declare(root, source, target);
    track(root);
    assert!(
        super::run(&["doctor", seat]).contains("true to the skeleton"),
        "fixture should stand true before the loose leaf"
    );

    std::fs::write(root.join("outside.rs"), "fn stray() {}\n").expect("stray");
    let outside = super::run(&["doctor", seat]);
    assert!(!outside.contains("untracked leaves"), "{outside}");

    std::fs::write(root.join("crates/tool/src/loose.rs"), "fn loose() {}\n").expect("loose");
    let held = super::run(&["doctor", seat]);
    assert!(
        held.contains("source crates/tool/src holds untracked leaves"),
        "{held}"
    );
    assert!(held.contains("crates/tool/src/loose.rs"), "{held}");

    let refused = super::run(&["document", seat]);
    assert!(
        !refused.contains("source crates/tool/src seal ="),
        "{refused}"
    );
    assert!(
        refused.contains("git add or ignore them first"),
        "{refused}"
    );

    track(root);
    let sealed = super::run(&["doctor", seat]);
    assert!(!sealed.contains("untracked leaves"), "{sealed}");
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
