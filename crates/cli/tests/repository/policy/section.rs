use super::{govern, policy, run};

#[test]
fn converged() {
    let scratch = tempfile::tempdir().expect("fixture");
    let root = scratch.path().join("plumb-policy-converged");
    std::fs::create_dir_all(root.join("apps/web/src/lib/components")).expect("components");
    std::fs::create_dir_all(root.join("skills/plumb")).expect("skills");
    govern(&root);
    std::fs::write(
        root.join("plumb.toml"),
        "[identity]\nns = \"probe\"\n\n[runseal]\n",
    )
    .expect("manifest");

    let clean = run(&root);
    for absent in [
        "no ectropy.toml",
        "missing ectropy",
        "unexpected ectropy",
        "ectropy limit",
    ] {
        assert!(!clean.contains(absent), "{clean}");
    }
    assert!(clean.contains("law       block=4 path=3"), "{clean}");

    let output = policy(&root, true);
    assert!(output.status.success(), "{output:?}");
    assert!(!root.join("ectropy.toml").exists());
    let text = std::fs::read_to_string(root.join("plumb.toml")).expect("manifest");
    assert!(!text.contains("[ectropy"), "{text}");
    assert!(text.contains("ns = \"probe\""), "{text}");
    assert!(text.contains("[runseal]"), "{text}");
}

#[test]
fn sparse() {
    let scratch = tempfile::tempdir().expect("fixture");
    let root = scratch.path().join("plumb-policy-sparse");
    std::fs::create_dir_all(root.join("apps/web/src/lib/components")).expect("components");
    std::fs::create_dir_all(root.join("crates/cli/src")).expect("crates");
    std::fs::write(root.join("apps/web/vite.config.ts"), "").expect("vite");
    std::fs::write(root.join("crates/cli/src/lib.rs"), "").expect("lib");
    govern(&root);
    std::fs::write(
        root.join("plumb.toml"),
        "[identity]\nns = \"probe\"\n\n[runseal]\n\n[[ectropy.boundary]]\npaths = [\"apps/web/vite.config.ts\"]\nallow = [\"path\"]\nnote = \"vite config\"\n\n[[ectropy.grant]]\nsyntax = \"environment\"\npaths = [\"apps/web/vite.config.ts\"]\n\n[[ectropy.grant]]\nsyntax = \"lock\"\npaths = [\"crates/cli/src/lib.rs\"]\n",
    )
    .expect("manifest");

    let clean = run(&root);
    for absent in [
        "no ectropy.toml",
        "missing ectropy",
        "unexpected ectropy",
        "ectropy limit",
        "boundary names",
    ] {
        assert!(!clean.contains(absent), "{clean}");
    }
    assert!(clean.contains("law       block=4 path=3"), "{clean}");
    assert!(clean.contains("environment"), "{clean}");
    assert!(clean.contains("lock"), "{clean}");
}

#[test]
fn reconcile() {
    let scratch = tempfile::tempdir().expect("fixture");
    let root = scratch.path().join("plumb-policy-reconcile");
    std::fs::create_dir_all(root.join("apps/web/src/lib/components")).expect("components");
    std::fs::create_dir_all(root.join("skills/plumb")).expect("skills");
    std::fs::write(root.join("apps/web/vite.config.ts"), "").expect("vite");
    govern(&root);
    std::fs::write(
        root.join("plumb.toml"),
        "[identity]\nns = \"probe\"\n\n[runseal]\n\n[ectropy.scan]\ninclude = [\"old\"]\n\n[ectropy.module]\nroots = [\"old\"]\n\n[ectropy.limit]\nblock = 99\n\n[ectropy.comment]\nallow = true\n\n[[ectropy.grant]]\nsyntax = \"test\"\npaths = [\"extra/**\"]\n\n[[ectropy.grant]]\nsyntax = \"environment\"\npaths = [\"apps/web/vite.config.ts\"]\n\n[[ectropy.grant]]\nsyntax = \"environment\"\npaths = [\"apps/web/vite.config.ts\", \"apps/web/extra.ts\"]\n\n[[ectropy.grant]]\nsyntax = \"lock\"\npaths = [\"apps/web/vite.config.ts\"]\n\n[[ectropy.ban]]\nsyntax = \"style\"\npaths = [\"apps/web/src/lib/components/**\", \"custom/**\"]\n\n[[ectropy.boundary]]\npaths = [\"apps/web/vite.config.ts\"]\nallow = [\"path\"]\nnote = \"vite config\"\n",
    )
    .expect("manifest");

    let output = policy(&root, true);
    assert!(output.status.success(), "{output:?}");
    assert!(!root.join("ectropy.toml").exists());
    let text = std::fs::read_to_string(root.join("plumb.toml")).expect("reconciled manifest");
    for stripped in [
        "[ectropy.scan]",
        "[ectropy.module]",
        "[ectropy.limit]",
        "[ectropy.comment]",
        "syntax = \"test\"",
        "apps/web/src/lib/components/**",
        "skills/**/*.md",
    ] {
        assert!(!text.contains(stripped), "should strip {stripped}:\n{text}");
    }
    for kept in [
        "ns = \"probe\"",
        "[runseal]",
        "apps/web/vite.config.ts",
        "apps/web/extra.ts",
        "syntax = \"environment\"",
        "syntax = \"lock\"",
        "custom/**",
        "allow = [\"path\"]",
    ] {
        assert!(text.contains(kept), "should keep {kept}:\n{text}");
    }
    assert_eq!(
        text.matches("syntax = \"environment\"").count(),
        1,
        "{text}"
    );

    let clean = run(&root);
    for absent in [
        "no ectropy.toml",
        "missing ectropy",
        "unexpected ectropy",
        "ectropy limit",
        "ectropy word",
        "ectropy comment",
    ] {
        assert!(!clean.contains(absent), "{clean}");
    }
}

#[test]
fn forms() {
    let scratch = tempfile::tempdir().expect("fixture");
    let root = scratch.path().join("plumb-policy-forms");
    std::fs::create_dir_all(&root).expect("fixture");
    govern(&root);
    for manifest in [
        "ectropy = { limit = { block = 99 } }\n\n[identity]\nns = \"probe\"\n",
        "ectropy.limit.block = 99\n\n[identity]\nns = \"probe\"\n",
    ] {
        std::fs::write(root.join("plumb.toml"), manifest).expect("manifest");
        let output = policy(&root, true);
        assert!(output.status.success(), "{manifest}\n{output:?}");
        let text = std::fs::read_to_string(root.join("plumb.toml")).expect("stripped");
        assert!(!text.contains("ectropy"), "{manifest}\n{text}");
        assert!(text.contains("ns = \"probe\""), "{manifest}\n{text}");
    }
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
    std::fs::create_dir_all(root.join("apps/web/src/lib/components")).expect("components");
    std::fs::create_dir_all(root.join("skills/plumb")).expect("skills");
    govern(&root);
    let kept = "# top comment\n[identity]\nns = \"probe\"\n\n[ectropy.scan]\ninclude = [\"old\"]\n\n[[ectropy.grant]]\nsyntax = \"lock\"\npaths = [\"crates/cli/src/lib.rs\"]\n\n# release comment\n[release]\nauthority = \"https://example.invalid\"\n";
    std::fs::write(root.join("plumb.toml"), kept).expect("manifest");
    assert!(policy(&root, true).status.success());
    let text = std::fs::read_to_string(root.join("plumb.toml")).expect("patched");
    for held in [
        "# top comment",
        "# release comment",
        "ns = \"probe\"",
        "syntax = \"lock\"",
    ] {
        assert!(text.contains(held), "{text}");
    }
    assert!(!text.contains("[ectropy.scan]"), "{text}");
    let headers: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('['))
        .collect();
    let identity = headers
        .iter()
        .position(|line| *line == "[identity]")
        .expect("identity");
    let release = headers
        .iter()
        .position(|line| *line == "[release]")
        .expect("release");
    let ectropy: Vec<usize> = headers
        .iter()
        .enumerate()
        .filter(|(_, line)| line.starts_with("[ectropy.") || line.starts_with("[[ectropy."))
        .map(|(at, _)| at)
        .collect();
    assert!(
        !ectropy.is_empty() && identity < ectropy[0] && *ectropy.last().unwrap() < release,
        "{text}"
    );
    assert_eq!(
        ectropy,
        (ectropy[0]..=ectropy[ectropy.len() - 1]).collect::<Vec<_>>(),
        "{text}"
    );

    let empty = "# top comment\n[identity]\nns = \"probe\"\n\n[ectropy.scan]\ninclude = [\"old\"]\n\n[[ectropy.ban]]\nsyntax = \"style\"\npaths = [\"apps/web/src/lib/components/**\"]\n\n# release comment\n[release]\nauthority = \"https://example.invalid\"\n";
    std::fs::write(root.join("plumb.toml"), empty).expect("manifest");
    assert!(policy(&root, true).status.success());
    let stripped = std::fs::read_to_string(root.join("plumb.toml")).expect("stripped");
    assert!(
        !stripped.contains("[ectropy") && !stripped.contains("[[ectropy"),
        "{stripped}"
    );
    for held in [
        "# top comment",
        "# release comment",
        "ns = \"probe\"",
        "authority = \"https://example.invalid\"",
    ] {
        assert!(stripped.contains(held), "{stripped}");
    }
}
