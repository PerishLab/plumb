use super::{govern, policy, run};

#[test]
fn overrides() {
    let scratch = tempfile::tempdir().expect("fixture");
    let root = scratch.path().join("plumb-policy-overrides");
    std::fs::create_dir_all(root.join("skills/plumb")).expect("skills");
    govern(&root);
    let cases = [
        (
            "[ectropy.limit]\nblock = 99\n",
            "ectropy limit block must be 4",
        ),
        (
            "[ectropy.limit]\ncombination = 999\n",
            "ectropy limit combination must be 3",
        ),
        (
            "[ectropy.limit]\ntuple = 999\n",
            "ectropy limit tuple must be 3",
        ),
        (
            "[ectropy.limit]\nschema = 999\n",
            "ectropy limit schema must be 2",
        ),
        (
            "[ectropy.comment]\nallow = true\n",
            "ectropy comment.allow must be false",
        ),
        (
            "[ectropy.scan]\ninclude = [\"old\"]\n",
            "unexpected ectropy scan include old",
        ),
        (
            "[[ectropy.grant]]\nsyntax = \"test\"\npaths = [\"extra/**\"]\n",
            "unexpected ectropy test grant extra/**",
        ),
        (
            "[[ectropy.boundary]]\npaths = [\"missing/file.rs\"]\nallow = [\"file\"]\nnote = \"gone\"\n",
            "boundary names missing/file.rs which does not exist",
        ),
    ];
    for (section, expected) in cases {
        let manifest = format!("[identity]\nns = \"probe\"\n\n[runseal]\n\n{section}");
        std::fs::write(root.join("plumb.toml"), manifest).expect("manifest");
        let judged = run(&root);
        assert!(judged.contains("out of true:"), "{section}\n{judged}");
        assert!(judged.contains(expected), "{section}\n{judged}");
    }
}

#[test]
fn malformed() {
    let scratch = tempfile::tempdir().expect("fixture");
    let root = scratch.path().join("plumb-policy-malformed");
    std::fs::create_dir_all(&root).expect("fixture");
    govern(&root);
    let cases = [
        ("[ectropy]\nbogus = 1\n", "unknown key `bogus`"),
        ("[ectropy.scan]\nbogus = []\n", "unknown key `scan.bogus`"),
        (
            "[ectropy.limit]\nblock = -1\n",
            "limit.block must be a non-negative integer",
        ),
        (
            "[ectropy.limit]\ncombination = -1\n",
            "limit.combination must be a non-negative integer",
        ),
        (
            "[ectropy.limit]\nblock = \"four\"\n",
            "limit.block must be a non-negative integer",
        ),
        ("[ectropy]\nscan = \"bad\"\n", "scan must be a table"),
        (
            "[ectropy.scan]\ninclude = \"crates/**/*.rs\"\n",
            "scan.include must be an array of strings",
        ),
        (
            "[ectropy.comment]\nallow = \"yes\"\n",
            "comment.allow must be a boolean",
        ),
        ("[ectropy.word]\nsingle = false\n", "unknown key `word`"),
        ("[ectropy.word]\nsingle = true\n", "unknown key `word`"),
        ("[ectropy.word]\n", "unknown key `word`"),
        (
            "[ectropy]\ngrant = [\"bad\"]\n",
            "grant must be an array of tables",
        ),
        (
            "[[ectropy.grant]]\nsyntax = \"bogus\"\npaths = [\"a/**\"]\n",
            "unknown grant syntax `bogus`",
        ),
        (
            "[[ectropy.grant]]\nsyntax = \"lock\"\npaths = []\n",
            "grant `lock` paths must not be empty",
        ),
        (
            "[[ectropy.ban]]\nsyntax = \"style\"\npaths = [\"\"]\n",
            "glob must not be empty",
        ),
        (
            "[[ectropy.ban]]\nsyntax = \"style\"\npaths = [\"../escape\"]\n",
            "invalid glob segment",
        ),
        (
            "[[ectropy.boundary]]\npaths = [\"a.rs\"]\nallow = [\"grant\"]\nnote = \"why\"\n",
            "boundary law `grant` admits no exemption",
        ),
        (
            "[[ectropy.boundary]]\npaths = [\"a.rs\"]\nallow = [\"word\"]\nnote = \"why\"\n",
            "boundary law `word` admits no exemption",
        ),
        (
            "[[ectropy.boundary]]\npaths = [\"a.rs\"]\nallow = [\"file\"]\nnote = \"   \"\n",
            "boundary note must not be empty",
        ),
        ("[ectropy.vocabulary]\n", "unknown key `vocabulary`"),
        (
            "[[ectropy.vocabulary.term]]\nname = \"vite_config\"\ndescription = \"fixture\"\n",
            "unknown key `vocabulary`",
        ),
    ];
    for (section, expected) in cases {
        let manifest = format!("[identity]\nns = \"probe\"\n\n[runseal]\n\n{section}");
        std::fs::write(root.join("plumb.toml"), &manifest).expect("manifest");
        let judged = run(&root);
        assert!(judged.contains("blind:"), "{section}\n{judged}");
        assert!(
            judged.contains("invalid [ectropy] in plumb.toml"),
            "{section}\n{judged}"
        );
        assert!(judged.contains(expected), "{section}\n{judged}");
        let rendered = policy(&root, true);
        assert!(!rendered.status.success(), "{section}\n{rendered:?}");
        let stderr = String::from_utf8_lossy(&rendered.stderr);
        assert!(stderr.contains(expected), "{section}\n{stderr}");
        assert_eq!(
            std::fs::read_to_string(root.join("plumb.toml")).expect("unchanged"),
            manifest
        );
    }
}
