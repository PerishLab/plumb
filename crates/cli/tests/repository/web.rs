use std::path::Path;
use std::process::Command;

fn run(root: &Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["doctor", root.to_str().expect("path should be utf8")])
        .output()
        .expect("plumb should run");
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn fixture(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&root);
    for path in [
        "apps/web/src/views",
        "apps/web/src/lib/components",
        "apps/web/src/lib/hooks",
        ".runseal/resources",
        ".forgejo/workflows",
    ] {
        std::fs::create_dir_all(root.join(path)).expect("fixture should be made");
    }
    std::fs::write(
        root.join("apps/web/package.json"),
        r#"{"name":"@specimen/web","scripts":{"build":"vite build"},"dependencies":{"@perish/design":"0.2.1","svelte":"5","vite":"8"},"devDependencies":{}}"#,
    )
    .expect("manifest should be written");
    std::fs::write(
        root.join("apps/web/vite.config.ts"),
        "import { design } from '@perish/design/vite'; design();",
    )
    .expect("vite config should be written");
    std::fs::write(
        root.join("apps/web/tsconfig.json"),
        r#"{"compilerOptions":{"types":["vite/client"]}}"#,
    )
    .expect("compiler config should be written");
    std::fs::write(
        root.join("apps/web/src/main.ts"),
        "declare module \"virtual:perish/views\";\nimport source from 'virtual:perish/views'; import { Views } from '@perish/design'; mount(Views, { props: { source } });",
    )
    .expect("entry should be written");
    std::fs::write(
        root.join("apps/web/src/views/index.svelte"),
        "export default function Home() { return null; }",
    )
    .expect("view should be written");
    std::fs::write(
        root.join(".forgejo/workflows/guard.yml"),
        "name: guard\njobs:\n  guard:\n    steps:\n      - run: pnpm --filter @specimen/web build\n",
    )
    .expect("guard should be written");
    root
}

#[test]
fn standalone() {
    let root = fixture("plumb-web-standalone");
    let out = run(&root);
    std::fs::remove_dir_all(&root).expect("fixture should be swept");
    assert!(!out.contains("[web]"), "{out}");
}

#[test]
fn hosted() {
    let root = fixture("plumb-web-hosted");
    std::fs::create_dir_all(root.join(".github/workflows")).expect("github should be made");
    std::fs::rename(
        root.join(".forgejo/workflows/guard.yml"),
        root.join(".github/workflows/quality.yml"),
    )
    .expect("guard should move");
    let out = run(&root);
    std::fs::remove_dir_all(&root).expect("fixture should be swept");
    assert!(!out.contains("[web]"), "{out}");
}
