use std::path::Path;

struct Seat<'a> {
    root: &'a Path,
    ns: &'a str,
}

impl Seat<'_> {
    fn command(&self, program: &str, args: &[&str]) {
        let output = crate::support::outside(program)
            .args(args)
            .current_dir(self.root)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn declare(&self, names: &[String]) {
        let packages = serde_json::to_string(names).unwrap();
        std::fs::write(
            self.root.join("plumb.toml"),
            format!(
                "[identity]\nns='{}'\n[dependency]\nroot=true\n[release]\nauthority='https://example.invalid'\n[release.cargo]\nregistry='perish'\npackages={packages}\n",
                self.ns
            ),
        )
        .unwrap();
        self.command("git", &["add", "."]);
    }

    fn doctor(&self, widths: Option<&str>) -> String {
        let home = crate::support::home();
        let path = match widths {
            Some(widths) => {
                let path = home.path().join("state").join("override.json");
                std::fs::write(&path, widths).unwrap();
                path
            }
            None => home.path().join("missing.json"),
        };
        let output = crate::support::outside(env!("CARGO_BIN_EXE_plumb"))
            .args(["doctor", self.root.to_str().unwrap()])
            .env("PLUMB_HOME", home.path())
            .env("PLUMB_FAMILY_WIDTHS", path)
            .env_remove("PLUMB_RELEASE_VERSION")
            .output()
            .unwrap();
        String::from_utf8_lossy(&output.stdout).to_string()
    }

    fn report(&self, names: &[String]) -> String {
        self.declare(names);
        self.doctor(Some(EXERCISED))
    }
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
    let seat = Seat { root: &root, ns };
    seat.command(
        "git",
        &[
            "remote",
            "add",
            "origin",
            &format!("https://github.com/PerishLab/{ns}.git"),
        ],
    );
    seat.command("cargo", &["generate-lockfile", "--offline"]);
    (scratch, root, names)
}

const EXERCISED: &str = "{\"cargo\":14,\"npm\":4}";

#[test]
fn capacity() {
    let (_scratch, root, names) = fixture("cargo-capacity", "foo", 14);
    let seat = Seat {
        root: &root,
        ns: "foo",
    };
    let allowed = seat.report(&names[..13]);
    assert!(!allowed.contains("this width has never run"), "{allowed}");
    assert!(!allowed.contains("and Plumb permits"), "{allowed}");
    let refused = seat.report(&names);
    assert!(
        refused.contains("the cargo attachment declares 14 packages and Plumb permits 13"),
        "{refused}"
    );
}

#[test]
fn product() {
    let (_scratch, root, names) = fixture("cargo-product", "radicula", 31);
    let seat = Seat {
        root: &root,
        ns: "radicula",
    };
    let allowed = seat.report(&names[..14]);
    assert!(!allowed.contains("this width has never run"), "{allowed}");
    assert!(!allowed.contains("and Plumb permits"), "{allowed}");
}

#[test]
fn noted() {
    let (_scratch, root, names) = fixture("cargo-noted", "radicula", 30);
    let seat = Seat {
        root: &root,
        ns: "radicula",
    };
    let report = seat.report(&names);
    assert!(
        report.contains("the family has released 14; this width has never run"),
        "{report}"
    );
}

#[test]
fn capped() {
    let (_scratch, root, names) = fixture("cargo-capped", "radicula", 31);
    let seat = Seat {
        root: &root,
        ns: "radicula",
    };
    let report = seat.report(&names);
    assert!(
        report.contains("the cargo attachment declares 31 packages and Plumb permits 30"),
        "{report}"
    );
    assert!(!report.contains("this width has never run"), "{report}");
}

#[test]
fn unread() {
    let (_scratch, root, names) = fixture("cargo-unread", "radicula", 14);
    let seat = Seat {
        root: &root,
        ns: "radicula",
    };
    seat.declare(&names);
    let report = seat.doctor(None);
    assert!(
        report.contains("the family's released width could not be read"),
        "{report}"
    );
    assert!(!report.contains("this width has never run"), "{report}");
}
