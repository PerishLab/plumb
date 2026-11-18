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

fn fixture(
    name: &str,
    ns: &str,
    total: usize,
) -> (tempfile::TempDir, std::path::PathBuf, Vec<String>) {
    let (scratch, root) = super::seat(name);
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers=['crates/*']\nresolver='3'\n",
    )
    .unwrap();
    let names = (1..=total)
        .map(|index| format!("{ns}-p{index}"))
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
            &format!("https://github.com/PerishLab/{ns}.git"),
        ],
    );
    command(&root, "cargo", &["generate-lockfile", "--offline"]);
    (scratch, root, names)
}

fn report(root: &Path, ns: &str, names: &[String], count: usize) -> String {
    let packages = serde_json::to_string(&names[..count]).unwrap();
    std::fs::write(
        root.join("plumb.toml"),
        format!(
            "[identity]\nns='{ns}'\n[dependency]\nroot=true\n[release]\nauthority='https://example.invalid'\n[release.cargo]\nregistry='perish'\npackages={packages}\n"
        ),
    )
    .unwrap();
    command(root, "git", &["add", "."]);
    crate::run(&["doctor", root.to_str().unwrap()])
}

#[test]
fn capacity() {
    let (_scratch, root, names) = fixture("cargo-capacity", "foo", 14);
    for count in [12, 13, 14] {
        let report = report(&root, "foo", &names, count);
        assert_eq!(
            report.contains("and Plumb permits"),
            count == 14,
            "{report}"
        );
        if count <= 13 {
            assert!(report.contains("the skeleton has released 2"), "{report}");
        } else {
            assert!(
                report.contains("the cargo attachment declares 14 packages and Plumb permits 13"),
                "{report}"
            );
        }
    }
}

#[test]
fn product() {
    let (_scratch, root, names) = fixture("cargo-product", "radicula", 31);
    for count in [13, 14, 30, 31] {
        let report = report(&root, "radicula", &names, count);
        assert_eq!(
            report.contains("and Plumb permits"),
            count == 31,
            "{report}"
        );
        if count == 31 {
            assert!(
                report.contains("the cargo attachment declares 31 packages and Plumb permits 30"),
                "{report}"
            );
        } else {
            assert!(report.contains("the skeleton has released 2"), "{report}");
        }
    }
}
