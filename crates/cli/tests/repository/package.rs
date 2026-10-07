use std::path::Path;
use std::process::Command;

const WILDCARD: &str = "wildcard into src";

const MANAGER: &str = "package.json declares packageManager";

#[test]
fn exports() {
    let root = specimen();
    for (exports, refused) in [
        (r#"{"./*":"./src/*"}"#, true),
        (
            r#"{"./lib/*":{"source":"./src/lib/*.ts","default":"./dist/lib/*.js"}}"#,
            true,
        ),
        (r#"{".":"./src/index.ts","./*":["./src/**/*.ts"]}"#, true),
        (r#"{"./*":"./*"}"#, true),
        (
            r#"{".":{"source":"./src/index.ts","default":"./dist/index.js"},"./hooks":"./src/lib/hooks/index.ts"}"#,
            false,
        ),
        (r#"{"./*":"./dist/*"}"#, false),
        (r#""./src/index.ts""#, false),
    ] {
        write(
            root.path(),
            "packages/kit/package.json",
            &format!(r#"{{"name":"@specimen/kit","exports":{exports}}}"#),
        );
        let out = doctor(root.path());
        assert_eq!(out.contains(WILDCARD), refused, "{exports}: {out}");
    }
}

#[test]
fn tests() {
    let root = specimen();
    write(root.path(), "packages/kit/src/x.test.ts", "");
    write(root.path(), "packages/kit/tests/y.test.ts", "");
    write(root.path(), "packages/kit/src/z.spec.tsx", "");
    write(root.path(), "packages/kit/index.test.js", "");
    git(root.path(), &["add", "."]);
    let out = doctor(root.path());
    for path in ["src/x.test.ts", "src/z.spec.tsx"] {
        assert!(
            out.contains(&format!(
                "test packages/kit/{path} sits outside packages/kit/tests/"
            )),
            "{out}"
        );
    }
    for path in ["tests/y.test.ts", "index.test.js"] {
        assert!(!out.contains(&format!("packages/kit/{path}")), "{out}");
    }
}

#[test]
fn engines() {
    let root = specimen();
    for (manifest, manager) in [
        (r#"{}"#, false),
        (r#"{"engines":{"node":">=24","pnpm":"^11.13.0"}}"#, false),
        (r#"{"engines":{"node":"24.18.0","pnpm":"11.13.0"}}"#, false),
        (r#"{"packageManager":"pnpm@11.13.0"}"#, true),
    ] {
        write(root.path(), "package.json", manifest);
        let out = doctor(root.path());
        assert!(!out.contains("engines"), "{manifest}: {out}");
        assert_eq!(out.contains(MANAGER), manager, "{manifest}: {out}");
    }
}

fn specimen() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("packages/kit/src")).unwrap();
    std::fs::create_dir_all(root.path().join("packages/kit/tests")).unwrap();
    write(
        root.path(),
        "packages/kit/package.json",
        r#"{"name":"@specimen/kit"}"#,
    );
    write(root.path(), "packages/kit/src/index.ts", "");
    git(root.path(), &["init", "-q"]);
    root
}

fn doctor(root: &Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .env("PLUMB_HOME", super::support::home().keep())
        .args(["doctor", root.to_str().expect("path should be utf8")])
        .output()
        .expect("plumb should run");
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn write(root: &Path, path: &str, text: &str) {
    std::fs::write(root.join(path), text).unwrap();
}

fn git(root: &Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .status()
            .expect("git")
            .success()
    );
}
