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
        ".runseal/wrappers",
    ] {
        std::fs::create_dir_all(root.join(path)).expect("fixture should be made");
    }
    std::fs::write(
        root.join("apps/web/package.json"),
        r#"{"name":"@specimen/web","scripts":{"build":"vite build"},"dependencies":{"@perish/react-components":"0.1.1","react":"19","vite":"8"},"devDependencies":{"@jsr/perish__vite-plugin-design":"0.1.1"}}"#,
    )
    .expect("manifest should be written");
    std::fs::write(
        root.join("apps/web/vite.config.ts"),
        "import { design } from '@jsr/perish__vite-plugin-design'; design();",
    )
    .expect("vite config should be written");
    std::fs::write(
        root.join("apps/web/tsconfig.json"),
        r#"{"compilerOptions":{"types":["@perish/react-components/client"]}}"#,
    )
    .expect("compiler config should be written");
    std::fs::write(
        root.join("apps/web/src/main.tsx"),
        "import source from 'virtual:perish/views'; import { Views } from '@perish/react-components'; <Views source={source} />;",
    )
    .expect("entry should be written");
    std::fs::write(
        root.join("apps/web/src/views/index.tsx"),
        "export default function Home() { return null; }",
    )
    .expect("view should be written");
    std::fs::write(
        root.join(".runseal/wrappers/guard.ts"),
        r#"["pnpm", ["--filter", "@specimen/web", "build"]]"#,
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
fn roles() {
    let root = fixture("plumb-web-roles");
    std::fs::write(root.join("apps/web/src/views/helper.ts"), "")
        .expect("helper should be written");
    std::fs::write(root.join("apps/web/src/lib/Panel.tsx"), "")
        .expect("component should be written");
    std::fs::write(root.join("apps/web/src/lib/hooks/Hush.ts"), "")
        .expect("hook should be written");
    let out = run(&root);
    std::fs::remove_dir_all(&root).expect("fixture should be swept");
    for line in [
        "web views only hold route tsx files",
        "web lib tsx must live under lib/components",
        "web convention paths must be lowercase",
        "web hooks must be lowercase use-*.ts files",
    ] {
        assert!(out.contains(&format!("{line} [web]")), "{out}");
    }
}
