use std::path::Path;

#[test]
fn budget() {
    let fixture = tempfile::tempdir().expect("fixture");
    let root = fixture.path();
    std::fs::create_dir_all(root.join("crates/tool/src")).expect("source seat");
    std::fs::create_dir_all(root.join("crates/tool/tests")).expect("test seat");
    brief(root, "one\ntwo\n", "three\n", "four\n");
    std::fs::write(root.join("crates/tool/src/main.rs"), "one\ntwo\nthree\n").expect("source");
    std::fs::write(root.join("crates/tool/tests/main.rs"), "ignored\n").expect("test");

    let out = crate::run(&["doctor", root.to_str().expect("utf8 root")]);
    assert!(
        out.contains("skill     tool source=3 budget=120 text=4 files=3"),
        "{out}"
    );
    assert!(out.contains("true to the skeleton"), "{out}");
}

#[test]
fn curve() {
    for (source, budget) in [(0, 120), (3_600, 120), (10_000, 200), (40_000, 400)] {
        case(source, budget);
    }
}

#[test]
fn exact() {
    let fixture = tempfile::tempdir().expect("fixture");
    brief(fixture.path(), "brief\n", "paths\n", "scenarios\n");
    let out = crate::run(&["doctor", fixture.path().to_str().expect("utf8 root")]);
    assert!(out.contains("true to the skeleton"), "{out}");
}

#[test]
fn open() {
    let fixture = tempfile::tempdir().expect("fixture");
    let root = fixture.path();
    std::fs::create_dir_all(root.join("skills/tool/references")).expect("reference seat");
    std::fs::write(root.join("skills/tool/SKILL.md"), "brief\n").expect("brief");
    std::fs::write(root.join("skills/tool/references/path.md"), "path\n").expect("path");

    let out = crate::run(&["doctor", root.to_str().expect("utf8 root")]);
    assert!(
        out.contains("missing regular files [PATHS.md, SCENARIOS.md]"),
        "{out}"
    );
    assert!(out.contains("extra root entries [references]"), "{out}");
}

#[test]
fn excess() {
    let fixture = tempfile::tempdir().expect("fixture");
    brief(fixture.path(), &"line\n".repeat(121), "", "");
    let out = crate::run(&["doctor", fixture.path().to_str().expect("utf8 root")]);
    assert!(
        out.contains("skill tool has 121 Markdown lines, above budget 120 for 0 source lines"),
        "{out}"
    );
}

#[test]
fn unread() {
    let fixture = tempfile::tempdir().expect("fixture");
    brief(fixture.path(), "brief\n", "paths\n", "scenarios\n");
    std::fs::write(fixture.path().join("skills/tool/PATHS.md"), [0xff]).expect("invalid text");
    let out = crate::run(&["doctor", fixture.path().to_str().expect("utf8 root")]);
    assert!(out.contains("PATHS.md is not UTF-8"), "{out}");
    assert!(out.contains("1 blind"), "{out}");
}

#[cfg(unix)]
#[test]
fn symlink() {
    let fixture = tempfile::tempdir().expect("fixture");
    brief(fixture.path(), "brief\n", "paths\n", "scenarios\n");
    let path = fixture.path().join("skills/tool/PATHS.md");
    std::fs::remove_file(&path).expect("remove path");
    std::os::unix::fs::symlink("SKILL.md", &path).expect("link path");
    let out = crate::run(&["doctor", fixture.path().to_str().expect("utf8 root")]);
    assert!(out.contains("missing regular files [PATHS.md]"), "{out}");
}

#[cfg(unix)]
#[test]
fn source() {
    let fixture = tempfile::tempdir().expect("fixture");
    let root = fixture.path();
    std::fs::create_dir_all(root.join("crates/tool/code")).expect("source seat");
    std::os::unix::fs::symlink("code", root.join("crates/tool/src")).expect("source link");
    brief(root, "brief\n", "paths\n", "scenarios\n");
    let out = crate::run(&["doctor", root.to_str().expect("utf8 root")]);
    assert!(out.contains("source seat"), "{out}");
    assert!(out.contains("is a symlink"), "{out}");
    assert!(out.contains("1 blind"), "{out}");
}

fn case(source: usize, budget: usize) {
    let fixture = tempfile::tempdir().expect("fixture");
    let root = fixture.path();
    std::fs::create_dir_all(root.join("crates/tool/src")).expect("source seat");
    brief(root, "brief\n", "paths\n", "scenarios\n");
    std::fs::write(
        root.join("crates/tool/src/main.rs"),
        "line\n".repeat(source),
    )
    .expect("source");

    let out = crate::run(&["doctor", root.to_str().expect("utf8 root")]);
    let want = format!("skill     tool source={source} budget={budget} text=3 files=3");
    assert!(out.contains(&want), "{out}");
}

fn brief(root: &Path, skill: &str, paths: &str, scenarios: &str) {
    std::fs::create_dir_all(root.join("skills/tool")).expect("skill seat");
    std::fs::write(root.join("skills/tool/SKILL.md"), skill).expect("skill");
    std::fs::write(root.join("skills/tool/PATHS.md"), paths).expect("paths");
    std::fs::write(root.join("skills/tool/SCENARIOS.md"), scenarios).expect("scenarios");
}
