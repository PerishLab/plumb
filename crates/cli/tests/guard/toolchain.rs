use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

const COOKBOOK: &str = "see: plumb cookbook env.toolchain-domain";

const ALIGNED: [(&str, &str); 4] = [
    ("cargo", "cargo 1.96.1 (ea2d97820 2026-06-26)"),
    ("rustc", "rustc 1.96.1 (31fca3adb 2026-06-26)"),
    ("node", "v24.18.0"),
    ("pnpm", "11.13.0"),
];

fn binary() -> &'static Path {
    static ROOT: OnceLock<tempfile::TempDir> = OnceLock::new();
    ROOT.get_or_init(|| {
        let root = tempfile::tempdir().expect("binary");
        let source = root.path().join("probe.rs");
        std::fs::write(&source, r#"fn main() { let body = std::fs::read_to_string(std::env::current_exe().unwrap().with_extension("txt")).unwrap(); let mut lines = body.lines(); let status = lines.next().unwrap().parse().unwrap(); println!("{}", lines.next().unwrap()); eprintln!("{}", lines.next().unwrap()); std::process::exit(status); }"#).unwrap();
        let output = plumb::config::current("rustc")
            .arg(source).arg("-o").arg(root.path().join("probe.exe"))
            .output().expect("rustc");
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        root
    }).path()
}

fn stub(dir: &Path, tool: &str, body: &str) {
    let name = if cfg!(windows) {
        format!("{tool}.exe")
    } else {
        tool.into()
    };
    let path = dir.join(name);
    std::fs::copy(binary().join("probe.exe"), &path).expect("native stub");
    std::fs::write(path.with_extension("txt"), format!("0\n{body}\n\n")).expect("stub output");
}

fn doctor(root: &Path, tools: &Path) -> String {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let mut paths = vec![tools.to_path_buf()];
    paths.extend(std::env::split_paths(&path));
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .env("PLUMB_HOME", super::support::home().keep())
        .env(
            "PATH",
            std::env::join_paths(paths).expect("path should join"),
        )
        .args(["doctor", root.to_str().expect("path should be utf8")])
        .output()
        .expect("plumb should run");
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn specimen(manifests: &[&str]) -> (tempfile::TempDir, tempfile::TempDir) {
    let root = tempfile::tempdir().expect("root");
    for name in manifests {
        std::fs::write(root.path().join(name), "{}\n").expect("manifest should be written");
    }
    let tools = tempfile::tempdir().expect("tools");
    for (tool, body) in ALIGNED {
        stub(tools.path(), tool, body);
    }
    (root, tools)
}

#[test]
fn aligned() {
    let (root, tools) = specimen(&["Cargo.toml", "package.json"]);
    let out = doctor(root.path(), tools.path());
    assert!(!out.contains(COOKBOOK), "{out}");
}

#[test]
fn diverged() {
    let (root, tools) = specimen(&["Cargo.toml", "package.json"]);
    stub(tools.path(), "rustc", "rustc 1.95.0 (abc 2026-05-01)");
    stub(tools.path(), "pnpm", "11.13.1");
    std::fs::write(
        tools.path().join("cargo.txt"),
        "1\n\nerror: rustup could not choose a version of cargo to run\n",
    )
    .unwrap();
    let out = doctor(root.path(), tools.path());
    for line in [
        "rustc reports 1.95.0, the domain runs 1.96.1",
        "pnpm reports 11.13.1, the domain runs 11.13.0",
        "cargo does not run (error: rustup could not choose a version of cargo to run), the domain runs 1.96.1",
    ] {
        assert!(
            out.contains(&format!("{line}; {COOKBOOK}")),
            "{line}: {out}"
        );
    }
    assert!(!out.contains("node reports"), "{out}");
}

#[test]
fn unused() {
    let (root, tools) = specimen(&[]);
    for tool in ["cargo", "rustc", "node", "pnpm"] {
        stub(tools.path(), tool, "0.0.1");
    }
    let out = doctor(root.path(), tools.path());
    assert!(!out.contains(COOKBOOK), "{out}");
}
