#[test]
fn budget() {
    let fixture = tempfile::tempdir().expect("fixture");
    let root = fixture.path();
    std::fs::create_dir_all(root.join("crates/tool/src")).expect("source seat");
    std::fs::create_dir_all(root.join("skills/tool")).expect("skill seat");
    std::fs::create_dir_all(root.join("crates/tool/tests")).expect("test seat");
    std::fs::create_dir_all(root.join("skills/tool/references")).expect("reference seat");
    std::fs::write(root.join("crates/tool/src/main.rs"), "one\ntwo\nthree\n").expect("source");
    std::fs::write(root.join("crates/tool/tests/main.rs"), "ignored\n").expect("test");
    std::fs::write(root.join("skills/tool/SKILL.md"), "one\ntwo\n").expect("brief");
    std::fs::write(root.join("skills/tool/references/path.md"), "three\n").expect("path");

    let out = crate::run(&["doctor", root.to_str().expect("utf8 root")]);
    assert!(
        out.contains("skill     tool source=3 budget=120 text=3 files=2"),
        "{out}"
    );
}

#[test]
fn curve() {
    for (source, budget) in [(0, 120), (3_600, 120), (10_000, 200), (40_000, 400)] {
        case(source, budget);
    }
}

fn case(source: usize, budget: usize) {
    let fixture = tempfile::tempdir().expect("fixture");
    let root = fixture.path();
    std::fs::create_dir_all(root.join("crates/tool/src")).expect("source seat");
    std::fs::create_dir_all(root.join("skills/tool")).expect("skill seat");
    std::fs::write(
        root.join("crates/tool/src/main.rs"),
        "line\n".repeat(source),
    )
    .expect("source");
    std::fs::write(root.join("skills/tool/SKILL.md"), "brief\n").expect("brief");

    let out = crate::run(&["doctor", root.to_str().expect("utf8 root")]);
    let want = format!("skill     tool source={source} budget={budget} text=1 files=1");
    assert!(out.contains(&want), "{out}");
}
