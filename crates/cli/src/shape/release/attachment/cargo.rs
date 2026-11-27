use super::super::Spec;
use std::path::Path;

const HEAD: &str = "[identity]\nns = \"locus\"\n[release]\nauthority = \"https://releases.locus.example\"\nbinaries = [\"locus\"]\ntargets = [\"x86_64-unknown-linux-gnu\"]\n[release.cargo]\nregistry = \"perish\"\npackages = [\"locus\", \"locus-relay\"]\n";

fn decode(text: &str) -> Result<Spec, String> {
    Spec::decode(Path::new("."), text, "fixture")
}

#[test]
fn omitted() {
    let spec = decode(HEAD).expect("default library targets");
    let cargo = spec.cargo.as_ref().expect("cargo");
    assert!(cargo.targets.is_none());
    assert_eq!(
        cargo.platforms(),
        vec![
            "x86_64-unknown-linux-gnu",
            "aarch64-apple-darwin",
            "x86_64-pc-windows-msvc",
        ]
    );
}

#[test]
fn explicit() {
    let narrow = format!("{HEAD}targets = [\"x86_64-unknown-linux-gnu\"]\n");
    let spec = decode(&narrow).expect("narrowed library targets");
    let cargo = spec.cargo.as_ref().expect("cargo");
    assert_eq!(cargo.platforms(), vec!["x86_64-unknown-linux-gnu"]);

    let wide = format!(
        "{HEAD}targets = [\"x86_64-unknown-linux-gnu\", \"aarch64-unknown-linux-musl\", \"aarch64-apple-darwin\", \"x86_64-pc-windows-msvc\"]\n"
    );
    let spec = decode(&wide).expect("wide library targets");
    assert_eq!(spec.cargo.as_ref().expect("cargo").platforms().len(), 4);
}

#[test]
fn refused() {
    for (targets, reason) in [
        ("[]", "at least one target"),
        (
            "[\"x86_64-unknown-linux-gnu\", \"x86_64-unknown-linux-gnu\"]",
            "duplicate Cargo target",
        ),
        ("[\"x86_64-apple-darwin\"]", "unsupported release target"),
        (
            "[\"aarch64-apple-darwin\", \"x86_64-pc-windows-msvc\"]",
            "must include x86_64-unknown-linux-gnu",
        ),
    ] {
        let text = format!("{HEAD}targets = {targets}\n");
        let error = decode(&text).expect_err(&text);
        assert!(error.contains(reason), "{error}");
    }
}
