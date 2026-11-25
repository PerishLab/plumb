use super::super::Spec;
use super::oci::{Input, Part};
use std::fs;
use std::path::Path;

const HEAD: &str = "[identity]\nns = \"ensign\"\n[release]\nauthority = \"https://releases.ensign.example\"\nbinaries = [\"ensign\", \"ensign-api\"]\ntargets = [\"x86_64-unknown-linux-gnu\", \"aarch64-apple-darwin\"]\n[release.binary.ensign-api]\ntargets = [\"x86_64-unknown-linux-gnu\"]\ninstall = false\n";
const PACKAGE: &str = r#"{"name":"ensign-web","scripts":{"build":"vite build"}}"#;
const DOCKER: &str = "FROM node:22\nCMD [\"node\", \"dist/.perish/server.mjs\"]\n";
const MULTI: &str = "[release.oci]\nregistry = \"ghcr.io\"\naccount = \"PerishLab\"\n[release.oci.components.web]\nimage = \"perishlab/ensign-web\"\nweb = \"apps/web\"\nfile = \"deploy/web.Dockerfile\"\n[release.oci.components.api]\nimage = \"perishlab/ensign-api\"\nbinary = \"ensign-api\"\n";

fn decode(root: &Path, text: &str) -> Result<Spec, String> {
    Spec::decode(root, text, "fixture")
}

fn refused(root: &Path, text: &str, reason: &str) {
    let error = decode(root, text).expect_err(text);
    assert!(error.contains(reason), "{error}");
}

fn part(name: &str, image: &str, file: &str, input: Input) -> Part {
    Part {
        name: name.into(),
        registry: "ghcr.io".into(),
        account: "PerishLab".into(),
        image: image.into(),
        file: file.into(),
        input,
    }
}

fn fixture() -> tempfile::TempDir {
    let scratch = tempfile::tempdir().expect("fixture");
    let root = scratch.path();
    fs::create_dir_all(root.join("apps/web")).expect("web dir");
    fs::create_dir_all(root.join("deploy")).expect("deploy dir");
    fs::write(root.join("apps/web/package.json"), PACKAGE).expect("package");
    fs::write(root.join("deploy/web.Dockerfile"), DOCKER).expect("dockerfile");
    fs::write(root.join("deploy/api.Dockerfile"), "FROM scratch\n").expect("api dockerfile");
    scratch
}

#[test]
fn multi() {
    let scratch = fixture();
    let root = scratch.path();
    let text = format!("{HEAD}{MULTI}");
    let spec = decode(root, &text).expect("multi oci");
    assert_eq!(spec.image(), None);
    assert_eq!(
        spec.placements(),
        vec!["oci api ensign-api", "oci web apps/web"]
    );
    let oci = spec.oci.as_ref().expect("oci");
    assert_eq!(
        oci.parts(&spec),
        vec![
            part(
                "api",
                "perishlab/ensign-api",
                "Containerfile",
                Input::Binary("ensign-api".into()),
            ),
            part(
                "web",
                "perishlab/ensign-web",
                "deploy/web.Dockerfile",
                Input::Web("apps/web".into()),
            ),
        ]
    );

    let worker = "[release.oci.components.worker]\nimage = \"perishlab/ensign-worker\"\n";
    let with = decode(root, &format!("{text}{worker}")).expect("source component");
    assert_eq!(with.image(), None);
    assert_eq!(
        with.placements(),
        vec![
            "oci api ensign-api",
            "oci web apps/web",
            "oci worker source",
        ]
    );
    let parts = with.oci.as_ref().expect("oci").parts(&with);
    assert_eq!(parts[2].input, Input::Source);
    assert_eq!(parts[2].file, "Containerfile");
}

#[test]
fn single() {
    let scratch = fixture();
    let root = scratch.path();
    let base = "[release.oci]\nregistry = \"ghcr.io\"\nimage = \"perishlab/ensign\"\naccount = \"PerishLab\"\n";
    let primary = decode(root, &format!("{HEAD}{base}")).expect("default");
    assert_eq!(primary.image(), Some("ensign"));
    assert_eq!(primary.placements(), vec!["oci ensign"]);
    assert_eq!(
        primary.oci.as_ref().expect("oci").parts(&primary),
        vec![part(
            "ensign",
            "perishlab/ensign",
            "Containerfile",
            Input::Binary("ensign".into()),
        )]
    );

    let named = decode(
        root,
        &format!("{HEAD}{base}binary = \"ensign-api\"\nfile = \"deploy/api.Dockerfile\"\n"),
    )
    .expect("named");
    assert_eq!(named.image(), Some("ensign-api"));
    assert_eq!(named.placements(), vec!["oci ensign-api"]);
    let parts = named.oci.as_ref().expect("oci").parts(&named);
    assert_eq!(parts[0].file, "deploy/api.Dockerfile");
    assert_eq!(parts[0].input, Input::Binary("ensign-api".into()));

    let web = decode(
        root,
        &format!("{HEAD}{base}web = \"apps/web\"\nfile = \"deploy/web.Dockerfile\"\n"),
    )
    .expect("web");
    assert_eq!(web.image(), None);
    assert_eq!(web.placements(), vec!["oci apps/web"]);
    assert_eq!(
        web.oci.as_ref().expect("oci").parts(&web)[0].input,
        Input::Web("apps/web".into())
    );

    let solo = decode(root, &format!("[identity]\nns = \"ensign\"\n{base}")).expect("solo");
    assert_eq!(solo.image(), None);
    assert!(solo.placements().is_empty());
    assert_eq!(
        solo.oci.as_ref().expect("oci").parts(&solo)[0].input,
        Input::Source
    );

    for (extra, reason) in [
        (
            "binary = \"ensign-api\"\nweb = \"apps/web\"\nfile = \"deploy/web.Dockerfile\"\n",
            "both binary and web",
        ),
        ("web = \"apps/web\"\n", "requires an explicit file"),
        (
            "file = \"deploy/missing.Dockerfile\"\n",
            "image file is absent",
        ),
    ] {
        refused(root, &format!("{HEAD}{base}{extra}"), reason);
    }
    refused(
        root,
        &format!("{HEAD}[release.oci]\nregistry = \"ghcr.io\"\naccount = \"PerishLab\"\n"),
        "one owner and one image",
    );
}

#[test]
fn refusals() {
    let scratch = fixture();
    let root = scratch.path();
    let base = format!("{HEAD}{MULTI}");

    for extra in [
        "image = \"perishlab/ensign\"\n",
        "binary = \"ensign-api\"\n",
        "file = \"deploy/api.Dockerfile\"\n",
        "web = \"apps/web\"\n",
    ] {
        let mixed = base.replace(
            "account = \"PerishLab\"\n",
            &format!("account = \"PerishLab\"\n{extra}"),
        );
        refused(root, &mixed, "cannot mix");
    }

    for (from, to, reason) in [
        (
            "image = \"perishlab/ensign-web\"",
            "image = \"PerishLab/ensign-api\"",
            "duplicate OCI image",
        ),
        (
            "[release.oci.components.api]",
            "[release.oci.components.API]",
            "invalid image component",
        ),
        (
            "[release.oci.components.api]",
            "[release.oci.components.bad_name]",
            "invalid image component",
        ),
        (
            "binary = \"ensign-api\"",
            "binary = \"ensign-api\"\nweb = \"apps/web\"\nfile = \"deploy/web.Dockerfile\"",
            "both binary and web",
        ),
        (
            "file = \"deploy/web.Dockerfile\"\n",
            "",
            "requires an explicit file",
        ),
        (
            "binary = \"ensign-api\"",
            "binary = \"ghost\"",
            "undeclared binary ghost",
        ),
        (
            "binary = \"ensign-api\"",
            "binary = \"ensign-api\"\ntag = \"latest\"",
            "unknown field",
        ),
        (
            "image = \"perishlab/ensign-api\"",
            "image = \"bare\"",
            "one owner and one image",
        ),
    ] {
        refused(root, &base.replace(from, to), reason);
    }

    let darwin = base.replace(
        "targets = [\"x86_64-unknown-linux-gnu\"]",
        "targets = [\"aarch64-apple-darwin\"]",
    );
    refused(root, &darwin, "does not target x86_64-unknown-linux-gnu");
}

#[test]
fn paths() {
    let scratch = fixture();
    let root = scratch.path();
    let base = format!("{HEAD}{MULTI}");

    for bad in [
        "../secret",
        "deploy/../web.Dockerfile",
        "./deploy/web.Dockerfile",
        "/etc/passwd",
        "deploy\\\\web.Dockerfile",
        "C:web.Dockerfile",
        "",
    ] {
        refused(
            root,
            &base.replace("deploy/web.Dockerfile", bad),
            "repository-relative",
        );
    }
    for bad in ["../web", "apps/../web", "./apps/web", "/apps/web"] {
        refused(root, &base.replace("apps/web", bad), "repository-relative");
    }
    for (from, to, reason) in [
        ("deploy/web.Dockerfile", "deploy", "regular file"),
        ("apps/web", "deploy/web.Dockerfile", "must be a directory"),
    ] {
        refused(root, &base.replace(from, to), reason);
    }

    for (json, reason) in [
        ("{broken", "invalid"),
        ("[]", "JSON object"),
        (r#"{"scripts":{"build":"vite"}}"#, "string name"),
        (r#"{"name":" ","scripts":{"build":"vite"}}"#, "string name"),
        (r#"{"name":"ensign-web"}"#, "scripts.build"),
        (r#"{"name":"ensign-web","scripts":{"build":""}}"#, "build"),
    ] {
        fs::write(root.join("apps/web/package.json"), json).expect("package");
        refused(root, &base, reason);
    }
    fs::remove_file(root.join("apps/web/package.json")).expect("remove package");
    refused(root, &base, "missing package.json");
}

#[cfg(unix)]
#[test]
fn symlinks() {
    use std::os::unix::fs::symlink;

    let scratch = fixture();
    let root = scratch.path();
    let base = format!("{HEAD}{MULTI}");

    symlink("web.Dockerfile", root.join("deploy/link.Dockerfile")).expect("file link");
    symlink("deploy", root.join("alias")).expect("dir link");
    symlink("web", root.join("apps/site")).expect("web link");
    for (from, to) in [
        ("deploy/web.Dockerfile", "deploy/link.Dockerfile"),
        ("deploy/web.Dockerfile", "alias/web.Dockerfile"),
        ("apps/web", "apps/site"),
    ] {
        refused(root, &base.replace(from, to), "symlink");
    }

    fs::write(root.join("apps/real.json"), PACKAGE).expect("real package");
    fs::remove_file(root.join("apps/web/package.json")).expect("remove");
    symlink("../real.json", root.join("apps/web/package.json")).expect("manifest link");
    refused(root, &base, "symlink");
}
