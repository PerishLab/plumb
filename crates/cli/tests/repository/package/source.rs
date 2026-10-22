use super::{doctor, git, specimen, write};
use std::path::Path;
use std::process::Command;

const SOURCE: &str = "package implementation";

#[test]
fn placement() {
    let root = specimen();
    let accepted = [
        "src/lib/render.ts",
        "src/skin.less",
        "tests/fixture.py",
        "dist/index.js",
        "target/output.rs",
        ".svelte-kit/generated.ts",
        "node_modules/dep/lib.ts",
        "vitest.config.ts",
        "vite.config.mjs",
        "svelte.config.js",
        "media/icon.svg",
    ];
    let refused = [
        "lib/render.ts",
        "build.ts",
        "tools/render.py",
        "lib/dist/escape.ts",
        "lib/vitest.config.ts",
        "vite.config.css",
        "test/case.ts",
    ];
    for file in accepted.iter().chain(&refused) {
        let path = root.path().join("packages/kit").join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "").unwrap();
    }
    git(root.path(), &["add", "--force", "."]);
    let out = doctor(root.path());
    for file in accepted {
        assert!(
            !out.contains(&format!("{SOURCE} packages/kit/{file} ")),
            "{out}"
        );
    }
    for file in refused {
        assert!(
            out.contains(&format!("{SOURCE} packages/kit/{file} ")),
            "{out}"
        );
    }
}

#[test]
fn policy() {
    let root = specimen();
    write(root.path(), "ectropy.toml", "");
    let mut doc = render(root.path());
    let roots = doc["module"]["roots"].as_array_mut().unwrap();
    assert!(roots.iter().all(|v| v.as_str() != Some("packages/*/lib")));
    assert!(roots.iter().any(|v| v.as_str() == Some("packages/*/src")));
    assert!(roots.iter().any(|v| v.as_str() == Some("packages/*/tests")));
    let canonical = toml::to_string(&doc).unwrap();
    write(root.path(), "ectropy.toml", &canonical);
    assert!(!doctor(root.path()).contains("ectropy module roots"));
    doc["module"]["roots"]
        .as_array_mut()
        .unwrap()
        .push("packages/*/lib".into());
    write(root.path(), "ectropy.toml", &toml::to_string(&doc).unwrap());
    assert!(!doctor(root.path()).contains("ectropy module roots"));
    std::fs::create_dir(root.path().join("packages/kit/lib")).unwrap();
    write(root.path(), "packages/kit/lib/render.ts", "");
    git(root.path(), &["add", "."]);
    assert!(doctor(root.path()).contains("package implementation packages/kit/lib/render.ts"));
    assert!(
        !render(root.path())["module"]["roots"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v.as_str() == Some("packages/*/lib"))
    );
    doc["module"]["roots"]
        .as_array_mut()
        .unwrap()
        .push("packages/*/other".into());
    write(root.path(), "ectropy.toml", &toml::to_string(&doc).unwrap());
    assert!(doctor(root.path()).contains("unexpected ectropy module roots packages/*/other"));
    write(
        root.path(),
        "ectropy.toml",
        &canonical.replace("packages/*/src", "packages/*/gone"),
    );
    assert!(doctor(root.path()).contains("missing ectropy module roots packages/*/src"));
}

fn render(root: &Path) -> toml::Value {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .env("PLUMB_HOME", super::super::support::home().keep())
        .args(["policy", root.to_str().unwrap(), "--write"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    toml::from_str(&std::fs::read_to_string(root.join("ectropy.toml")).unwrap()).unwrap()
}
