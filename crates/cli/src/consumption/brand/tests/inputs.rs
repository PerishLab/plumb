use super::super as brand;

fn world() -> (tempfile::TempDir, Vec<brand::source::App>) {
    let root = tempfile::tempdir().unwrap();
    let base = root.path();
    for dir in ["src", "crates", "public"] {
        std::fs::create_dir(base.join(dir)).unwrap();
    }
    std::fs::write(base.join("src/entry.svelte"), "<Hero />").unwrap();
    std::fs::write(base.join("crates/lib.rs"), "fn original(){}").unwrap();
    std::fs::write(base.join("public/image.svg"), "original image").unwrap();
    for name in [
        "package.json",
        "pnpm-lock.yaml",
        "index.html",
        "vite.config.ts",
    ] {
        std::fs::write(base.join(name), "tracked input").unwrap();
    }
    for args in [vec!["init", "-q"], vec!["add", "."]] {
        assert!(
            plumb::config::detached("git")
                .args(args)
                .current_dir(base)
                .status()
                .unwrap()
                .success()
        );
    }
    let apps = vec![brand::source::App {
        mode: brand::Mode::Html,
        config: "vite.config.ts".into(),
        path: "".into(),
        setup: crate::shape::web::Setup {
            options: serde_json::json!({}),
            base: "/".into(),
            output: "dist".into(),
            problem: None,
        },
    }];
    (root, apps)
}

#[test]
fn inputs() {
    let (root, apps) = world();
    let base = root.path();
    let source = brand::source::Source(base);
    let before = source.inputs(&apps).unwrap();
    std::fs::write(base.join("src/entry.svelte"), "<Hero mark=\"fixture\" />").unwrap();
    let changed = source.inputs(&apps).unwrap();
    assert_ne!(before, changed);
    std::fs::write(base.join("crates/lib.rs"), "fn unrelated_rust_change(){}").unwrap();
    assert_eq!(changed, source.inputs(&apps).unwrap());
    std::fs::write(base.join("public/image.svg"), "changed image").unwrap();
    assert_ne!(changed, source.inputs(&apps).unwrap());
    std::fs::write(base.join("public/untracked.svg"), "unknown").unwrap();
    assert!(source.inputs(&apps).is_err());
}

#[test]
fn required() {
    for name in [
        "package.json",
        "pnpm-lock.yaml",
        "index.html",
        "vite.config.ts",
    ] {
        let (root, apps) = world();
        assert!(
            plumb::config::detached("git")
                .args(["rm", "--cached", name])
                .current_dir(root.path())
                .status()
                .unwrap()
                .success()
        );
        assert!(
            brand::source::Source(root.path()).inputs(&apps).is_err(),
            "{name}"
        );
    }
}

#[test]
fn workspace() {
    let (root, apps) = world();
    let path = root.path().join("pnpm-workspace.yaml");
    std::fs::write(path, "packages: [apps/*]").unwrap();
    let source = brand::source::Source(root.path());
    assert!(source.inputs(&apps).is_err());
    assert!(
        plumb::config::detached("git")
            .args(["add", "pnpm-workspace.yaml"])
            .current_dir(root.path())
            .status()
            .unwrap()
            .success()
    );
    source.inputs(&apps).unwrap();
}
