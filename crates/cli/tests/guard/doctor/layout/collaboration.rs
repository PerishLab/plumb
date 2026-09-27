use super::{DECLARED, report, seated, track};

const GITHUB: &str = r#"
[[layout.seat]]
path = ".github"
note = "repository metadata outside the inherited collaboration templates"
"#;

const POLICY: &str = "https://github.com/PerishLab/.github/blob/main/GOVERNANCE.md";

#[test]
fn reserved() {
    for path in [
        ".github/ISSUE_TEMPLATE/feature.yml",
        ".github/PULL_REQUEST_TEMPLATE",
        ".github/PULL_REQUEST_TEMPLATE.md",
        ".github/PULL_REQUEST_TEMPLATE/feature.md",
    ] {
        let seat = seated(&format!("{DECLARED}\n{GITHUB}"));
        let target = seat.path().join(path);
        std::fs::create_dir_all(target.parent().expect("template parent")).expect("template seat");
        std::fs::write(&target, "local override\n").expect("template");
        track(seat.path());
        let held = report(seat.path());
        assert!(held.contains(path), "{held}");
        assert!(held.contains(POLICY), "{held}");
    }
}

#[test]
fn metadata() {
    let seat = seated(&format!("{DECLARED}\n{GITHUB}"));
    for path in [".github/CODEOWNERS", ".github/workflows/quality.yml"] {
        let target = seat.path().join(path);
        std::fs::create_dir_all(target.parent().expect("metadata parent")).expect("metadata seat");
        std::fs::write(&target, "owned metadata\n").expect("metadata");
    }
    track(seat.path());
    let held = report(seat.path());
    assert!(!held.contains(POLICY), "{held}");
    assert!(
        !held.contains("directory .github sits in no declared seat"),
        "{held}"
    );
}

#[test]
fn authority() {
    let seat = seated(&format!("{DECLARED}\n{GITHUB}"));
    std::fs::write(
        seat.path().join("Cargo.toml"),
        "[package]\nname = \"policy\"\nversion = \"0.0.0\"\nrepository = \"https://github.com/PerishLab/.github\"\n",
    )
    .expect("authority identity");
    let path = ".github/ISSUE_TEMPLATE/feature.yml";
    let target = seat.path().join(path);
    std::fs::create_dir_all(target.parent().expect("template parent")).expect("template seat");
    std::fs::write(&target, "organization form\n").expect("template");
    track(seat.path());
    let held = report(seat.path());
    assert!(!held.contains(POLICY), "{held}");
}
