use plumb::identity::{Binding, Region};
use std::{fs, path::Path, process::Command};

fn output(command: &mut Command) -> String {
    let result = command.output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    String::from_utf8(result.stdout).unwrap()
}

pub fn archive(command: &impl Fn() -> Command, path: &Path, version: &str) {
    let shown = output(command().args(["release", "show", "--marker", version, "--held"]));
    let marker: serde_json::Value = serde_json::from_str(&shown).unwrap();
    let verified = output(command().args(["release", "verify", "--marker", version]));
    let digest = verified
        .trim()
        .rsplit_once('(')
        .unwrap()
        .1
        .trim_end_matches(')');
    let commit = marker["commit"].as_str().unwrap();
    let root = tempfile::tempdir().unwrap();
    let region = Region::new("PROBE", Some(commit), Some("x86_64-unknown-linux-gnu"));
    let source = format!(
        "#[used]\n#[cfg_attr(target_vendor=\"apple\", unsafe(link_section=\"__DATA,__plumbid\"))]\n#[cfg_attr(not(target_vendor=\"apple\"), unsafe(link_section=\".plumbid\"))]\nstatic REGION: [u8;4096] = {:?};\nfn main() {{ std::hint::black_box(unsafe {{ std::ptr::read_volatile(&raw const REGION) }}); println!(\"probe {version}\"); }}\n",
        region.read()
    );
    fs::write(root.path().join("probe.rs"), source).unwrap();
    let binary = root.path().join("probe");
    output(
        Command::new("rustc")
            .arg(root.path().join("probe.rs"))
            .args(["--edition=2024", "-o"])
            .arg(&binary),
    );
    let original = fs::read(&binary).unwrap();
    let identity = Binding {
        product: "probe".into(),
        marker: version.into(),
        digest: digest.into(),
        commit: commit.into(),
        workload: plumb::depot::sha(&original),
    };
    fs::write(
        &binary,
        plumb::identity::bind(&original, &identity).unwrap(),
    )
    .unwrap();
    output(
        Command::new("tar")
            .arg("-C")
            .arg(root.path())
            .arg("-czf")
            .arg(path)
            .arg("probe"),
    );
}
