use std::path::Path;

fn command(root: &Path, program: &str, args: &[&str]) {
    let output = crate::support::outside(program)
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn capacity() {
    let (_scratch, root) = super::seat("cargo-capacity");
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers=['crates/*']\nresolver='3'\n",
    )
    .unwrap();
    let names = (1..=12)
        .map(|index| format!("foo-p{index}"))
        .collect::<Vec<_>>();
    for name in &names {
        let path = root.join("crates").join(name);
        std::fs::create_dir_all(path.join("src")).unwrap();
        std::fs::write(
            path.join("Cargo.toml"),
            format!("[package]\nname='{name}'\nversion='0.0.0'\nedition='2024'\n"),
        )
        .unwrap();
        std::fs::write(path.join("src/lib.rs"), "").unwrap();
    }
    command(
        &root,
        "git",
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/PerishLab/foo.git",
        ],
    );
    command(&root, "cargo", &["generate-lockfile", "--offline"]);
    for count in [11, 12] {
        let packages = serde_json::to_string(&names[..count]).unwrap();
        std::fs::write(
            root.join("plumb.toml"),
            format!(
                "[identity]\nns='foo'\n[dependency]\nroot=true\n[release]\nauthority='https://example.invalid'\n[release.cargo]\nregistry='perish'\npackages={packages}\n"
            ),
        )
        .unwrap();
        command(&root, "git", &["add", "."]);
        let report = crate::run(&["doctor", root.to_str().unwrap()]);
        assert_eq!(
            report.contains("and Plumb permits"),
            count == 12,
            "{report}"
        );
        if count == 11 {
            assert!(report.contains("the skeleton has released 2"), "{report}");
        } else {
            assert!(
                report.contains("the cargo attachment declares 12 packages and Plumb permits 11"),
                "{report}"
            );
        }
    }
}
