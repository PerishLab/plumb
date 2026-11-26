use super::{govern, policy, run};

#[test]
fn embedded() {
    let scratch = tempfile::tempdir().expect("fixture");
    let root = scratch.path().join("plumb-policy-embedded");
    std::fs::create_dir_all(root.join("apps/web/src/lib/components")).expect("components");
    std::fs::create_dir_all(root.join("skills/plumb")).expect("skills");
    govern(&root);
    std::fs::write(
        root.join("plumb.toml"),
        "[identity]\nns = \"probe\"\n\n[runseal]\n",
    )
    .expect("manifest");
    let missing = run(&root);
    assert!(missing.contains("no ectropy.toml"), "{missing}");

    std::fs::write(
        root.join("plumb.toml"),
        "[identity]\nns = \"probe\"\n\n[runseal]\n\n[ectropy.scan]\ninclude = [\"old\"]\n\n[[ectropy.vocabulary.term]]\nname = \"vite_config\"\ndescription = \"fixture\"\n",
    )
    .expect("manifest");
    let drifted = run(&root);
    assert!(!drifted.contains("no ectropy.toml"), "{drifted}");
    assert!(
        drifted.contains("missing ectropy style ban apps/web/src/lib/components/**"),
        "{drifted}"
    );

    let output = policy(&root, true);
    assert!(output.status.success(), "{output:?}");
    assert!(!root.join("ectropy.toml").exists());
    let text = std::fs::read_to_string(root.join("plumb.toml")).expect("reconciled manifest");
    assert!(text.contains("ns = \"probe\""), "{text}");
    assert!(text.contains("[runseal]"), "{text}");
    assert!(text.contains("apps/web/src/lib/components/**"), "{text}");
    assert!(text.contains("skills/**/*.md"), "{text}");
    assert!(text.contains("name = \"vite_config\""), "{text}");

    let clean = run(&root);
    assert!(!clean.contains("no ectropy.toml"), "{clean}");
    assert!(!clean.contains("missing ectropy"), "{clean}");
    assert!(!clean.contains("unexpected ectropy"), "{clean}");
    assert!(!clean.contains("ectropy limit"), "{clean}");
}

#[test]
fn conflict() {
    let scratch = tempfile::tempdir().expect("fixture");
    let root = scratch.path().join("plumb-policy-conflict");
    std::fs::create_dir_all(&root).expect("fixture");
    govern(&root);
    std::fs::write(root.join("ectropy.toml"), "[limit]\nblock = 4\n").expect("legacy");
    std::fs::write(root.join("plumb.toml"), "[ectropy.limit]\nblock = 4\n").expect("section");

    let judged = run(&root);
    assert!(judged.contains("blind:"), "{judged}");
    assert!(
        judged.contains("both ectropy.toml and [ectropy] in plumb.toml"),
        "{judged}"
    );

    let rendered = policy(&root, false);
    assert!(!rendered.status.success(), "{rendered:?}");
    let stderr = String::from_utf8_lossy(&rendered.stderr);
    assert!(
        stderr.contains("both ectropy.toml and [ectropy] in plumb.toml"),
        "{stderr}"
    );
}

#[test]
fn runseal() {
    let scratch = tempfile::tempdir().expect("fixture");
    let root = scratch.path().join("plumb-runseal-conflict");
    std::fs::create_dir_all(&root).expect("fixture");
    govern(&root);
    std::fs::write(root.join("runseal.toml"), "").expect("legacy");
    std::fs::write(root.join("plumb.toml"), "[runseal]\n").expect("section");
    let clash = run(&root);
    assert!(clash.contains("blind:"), "{clash}");
    assert!(
        clash.contains("both runseal.toml and [runseal] in plumb.toml"),
        "{clash}"
    );

    std::fs::remove_file(root.join("runseal.toml")).expect("remove legacy");
    std::fs::write(root.join("plumb.toml"), "runseal = \"bad\"\n").expect("scalar");
    let scalar = run(&root);
    assert!(scalar.contains("blind:"), "{scalar}");
    assert!(
        scalar.contains("cannot read [runseal] in plumb.toml"),
        "{scalar}"
    );
}

#[test]
fn contiguous() {
    let scratch = tempfile::tempdir().expect("fixture");
    let root = scratch.path().join("plumb-policy-contiguous");
    std::fs::create_dir_all(root.join("skills/plumb")).expect("skills");
    govern(&root);
    let manifest = "# top comment\n[identity]\nns = \"probe\"\n\n[ectropy.scan]\ninclude = [\"old\"]\n\n# release comment\n[release]\nauthority = \"https://example.invalid\"\n";
    std::fs::write(root.join("plumb.toml"), manifest).expect("manifest");
    let output = policy(&root, true);
    assert!(output.status.success(), "{output:?}");
    let text = std::fs::read_to_string(root.join("plumb.toml")).expect("patched");
    assert!(text.contains("# top comment"), "{text}");
    assert!(text.contains("# release comment"), "{text}");
    assert!(text.contains("ns = \"probe\""), "{text}");
    assert!(
        text.contains("authority = \"https://example.invalid\""),
        "{text}"
    );
    let headers: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('['))
        .collect();
    let identity = headers
        .iter()
        .position(|line| *line == "[identity]")
        .expect("identity header");
    let release = headers
        .iter()
        .position(|line| *line == "[release]")
        .expect("release header");
    let ectropy: Vec<usize> = headers
        .iter()
        .enumerate()
        .filter(|(_, line)| line.starts_with("[ectropy.") || line.starts_with("[[ectropy."))
        .map(|(at, _)| at)
        .collect();
    assert!(!ectropy.is_empty(), "{text}");
    assert!(identity < ectropy[0], "{headers:?}\n{text}");
    assert!(*ectropy.last().unwrap() < release, "{headers:?}\n{text}");
    assert_eq!(
        ectropy,
        (ectropy[0]..=ectropy[ectropy.len() - 1]).collect::<Vec<_>>(),
        "ectropy sections must remain contiguous:\n{text}"
    );
}
