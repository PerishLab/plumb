use super::Spec;
use std::path::Path;

const HEAD: &str = "[release]\nproduct = \"santi\"\nauthority = \"https://releases.santi.example\"\nbinaries = [\"santi\", \"santi-api\"]\n";
const UNION: &str = "targets = [\"x86_64-unknown-linux-gnu\", \"aarch64-apple-darwin\"]\n";
const NARROW: &str =
    "[release.binary.santi-api]\ntargets = [\"x86_64-unknown-linux-gnu\"]\ninstall = false\n";
const DEB: &str = "[release.deb]\nbinary = \"santi-api\"\nroot = \"packaging/deb\"\n";
const CONTROL: &str = "Package: santi-api\nVersion: __VERSION__\nArchitecture: amd64\n";
const UNIT: &str = "[Service]\nExecStart=/usr/bin/santi-api serve\n";

fn decode(root: &Path, text: &str) -> Result<Spec, String> {
    Spec::decode(root, text, "fixture")
}

fn refused(root: &Path, text: &str, reason: &str) {
    let error = decode(root, text).expect_err(text);
    assert!(error.contains(reason), "{error}");
}

fn packaged(root: &Path, control: &str, unit: &str) {
    let seat = root.join("packaging/deb");
    let units = seat.join("root/lib/systemd/system");
    let _ = std::fs::remove_dir_all(&seat);
    std::fs::create_dir_all(&units).expect("unit seat");
    std::fs::write(seat.join("control"), control).expect("control");
    std::fs::write(seat.join("postinst"), "#!/bin/sh\n").expect("postinst");
    if !unit.is_empty() {
        std::fs::write(units.join("santi-api.service"), unit).expect("unit");
    }
}

#[test]
fn executables() {
    let root = Path::new(".");
    let spec = decode(root, &format!("{HEAD}{UNION}{NARROW}")).expect("narrowed");
    let santi = spec.executable("santi").expect("santi");
    let api = spec.executable("santi-api").expect("api");
    assert!(santi.install && santi.carries("aarch64-apple-darwin"));
    assert!(!api.install && !api.carries("aarch64-apple-darwin"));
    assert_eq!(spec.primary(), Some("santi"));

    let unknown = "[release.binary.ghost]\ninstall = false\n";
    refused(
        root,
        &format!("{HEAD}{UNION}{unknown}"),
        "names no declared binary",
    );
    let outside = "[release.binary.santi-api]\ntargets = [\"x86_64-pc-windows-msvc\"]\n";
    refused(
        root,
        &format!("{HEAD}{UNION}{outside}"),
        "is not a release target",
    );
    let empty = "[release.binary.santi-api]\ntargets = []\n";
    refused(
        root,
        &format!("{HEAD}{UNION}{empty}"),
        "at least one target",
    );
}

#[test]
fn windows() {
    let root = Path::new(".");
    let union = "targets = [\"x86_64-unknown-linux-gnu\", \"x86_64-pc-windows-msvc\"]\n";
    refused(root, &format!("{HEAD}{union}"), "exactly one binary");
    let narrowed = "[release.binary.santi-api]\ntargets = [\"x86_64-unknown-linux-gnu\"]\n";
    decode(root, &format!("{HEAD}{union}{narrowed}")).expect("one binary per Windows target");
}

#[test]
fn image() {
    let root = Path::new(".");
    let oci = "[release.oci]\nregistry = \"ghcr.io\"\nimage = \"perishlab/santi\"\naccount = \"PerishLab\"\n";
    let held = decode(root, &format!("{HEAD}{UNION}{NARROW}{oci}")).expect("default image");
    assert_eq!(held.image(), Some("santi"));
    let named = format!("{HEAD}{UNION}{NARROW}{oci}binary = \"santi-api\"\n");
    assert_eq!(
        decode(root, &named).expect("named").image(),
        Some("santi-api")
    );
    let ghost = format!("{HEAD}{UNION}{oci}binary = \"ghost\"\n");
    refused(root, &ghost, "names undeclared binary ghost");
    let darwin = "[release.binary.santi-api]\ntargets = [\"aarch64-apple-darwin\"]\n";
    let foreign = format!("{HEAD}{UNION}{darwin}{oci}binary = \"santi-api\"\n");
    refused(root, &foreign, "does not target x86_64-unknown-linux-gnu");
}

#[test]
fn deb() {
    let fixture = tempfile::tempdir().expect("fixture");
    let root = fixture.path();
    let whole = format!("{HEAD}{UNION}{NARROW}{DEB}");
    refused(root, &whole, "no control template");
    packaged(root, CONTROL, UNIT);
    let spec = decode(root, &whole).expect("deb");
    assert!(spec.surface().contains(&"deb"));

    let ghost = DEB.replace("santi-api", "ghost");
    refused(
        root,
        &format!("{HEAD}{UNION}{ghost}"),
        "undeclared binary ghost",
    );
    let darwin = "[release.binary.santi-api]\ntargets = [\"aarch64-apple-darwin\"]\n";
    refused(
        root,
        &format!("{HEAD}{UNION}{darwin}{DEB}"),
        "does not target",
    );
    let outside = DEB.replace("packaging/deb", "../deb");
    refused(
        root,
        &format!("{HEAD}{UNION}{outside}"),
        "repository-relative",
    );

    packaged(root, &CONTROL.replace("__VERSION__", "1.0.0"), UNIT);
    refused(root, &whole, "__VERSION__ 0 times");
    packaged(root, &format!("{CONTROL}Description: __VERSION__\n"), UNIT);
    refused(root, &whole, "__VERSION__ 2 times");
    packaged(
        root,
        &CONTROL.replace("Package: santi-api", "Package: santi"),
        UNIT,
    );
    refused(root, &whole, "Package is santi;");
    packaged(root, CONTROL, "");
    refused(root, &whole, "no systemd unit");
    packaged(
        root,
        CONTROL,
        &UNIT.replace("santi-api serve", "santi-api-old"),
    );
    refused(root, &whole, "runs /usr/bin/santi-api");
}
