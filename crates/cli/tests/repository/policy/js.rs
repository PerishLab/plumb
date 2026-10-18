use std::path::Path;
use std::process::Command;

const SEATS: [(&str, &str); 3] = [
    ("apps/list", "apps"),
    ("packages/plumb", "packages"),
    ("tools/chromium", "tools"),
];

const FAMILY: [&str; 6] = ["cjs", "cts", "js", "jsx", "mjs", "mts"];

fn carried(name: &str) -> String {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .expect("git runs");
    assert!(output.status.success());
    let root = String::from_utf8(output.stdout).expect("git path");
    let path = Path::new(root.trim()).join("crates/cli/rules").join(name);
    std::fs::read_to_string(path).expect("carried rules")
}

fn write(root: &Path, files: &[String]) {
    for file in files {
        let path = root.join(file);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("source seat");
        std::fs::write(path, "").expect("source file");
    }
}

fn render(tracked: &[String], loose: &[String], scan: &str) -> toml::Value {
    let root = tempfile::tempdir().expect("repository");
    let home = tempfile::tempdir().expect("rules home");
    super::govern(root.path());
    write(root.path(), tracked);
    let status = Command::new("git")
        .arg("-C")
        .arg(root.path())
        .args(["add", "--all"])
        .status()
        .expect("git runs");
    assert!(status.success(), "fixture files should be tracked");
    write(root.path(), loose);
    super::stock(
        home.path(),
        super::LIMIT,
        scan,
        &carried("suites/shape.toml"),
    );
    let outputs = ["policy", "doctor"].map(|verb| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_plumb"));
        command
            .arg(verb)
            .arg(root.path())
            .env("PLUMB_HOME", home.path());
        if verb == "policy" {
            command.arg("--write");
        }
        command.output().expect("plumb runs")
    });
    let judged = String::from_utf8_lossy(&outputs[1].stdout).to_string();
    assert!(outputs[0].status.success(), "{:?}", outputs[0]);
    assert!(!judged.contains("missing ectropy"), "{judged}");
    assert!(!judged.contains("unexpected ectropy"), "{judged}");
    let text = std::fs::read_to_string(root.path().join("ectropy.toml")).expect("policy");
    toml::from_str(&text).expect("policy parses")
}

fn strings(policy: &toml::Value, table: &str, key: &str) -> Vec<String> {
    policy[table][key]
        .as_array()
        .map(|values| {
            values
                .iter()
                .filter_map(|item| item.as_str())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn earned(policy: &toml::Value, seat: &str) -> bool {
    let held = strings(policy, "scan", "include");
    let all = FAMILY
        .iter()
        .all(|suffix| held.contains(&format!("{seat}/**/*.{suffix}")));
    let any = FAMILY
        .iter()
        .any(|suffix| held.contains(&format!("{seat}/**/*.{suffix}")));
    assert_eq!(all, any, "{held:?}");
    all
}

#[test]
fn selected() {
    for (dir, seat) in SEATS {
        let policy = render(
            &[format!("{dir}/scenario.js")],
            &[],
            &carried("atoms/scan.toml"),
        );
        assert!(earned(&policy, seat), "{policy}");
    }
}

#[test]
fn typed() {
    for (dir, seat) in SEATS {
        for suffix in ["mts", "cts"] {
            let file = format!("{dir}/main.{suffix}");
            let policy = render(&[file], &[], &carried("atoms/scan.toml"));
            assert!(earned(&policy, seat), "{policy}");
        }
    }
}

#[test]
fn absent() {
    for (dir, seat) in SEATS {
        let source = format!("{dir}/src/main.ts");
        let policy = render(&[source], &[], &carried("atoms/scan.toml"));
        assert!(!earned(&policy, seat), "{policy}");
        assert!(!strings(&policy, "scan", "exclude").contains(&"tools/*/resources/**".to_string()));
    }
}

#[test]
fn untracked() {
    for (dir, seat) in SEATS {
        let loose = [
            format!("{dir}/node_modules/vite/index.js"),
            format!("{dir}/dist/index.mjs"),
        ];
        let policy = render(
            &[format!("{dir}/src/main.ts")],
            &loose,
            &carried("atoms/scan.toml"),
        );
        assert!(!earned(&policy, seat), "{policy}");
    }
}

#[test]
fn derived() {
    let scan = carried("atoms/scan.toml");
    let family = FAMILY
        .map(|suffix| format!("  \"tools/**/*.{suffix}\",\n"))
        .concat();
    assert!(scan.contains(&family), "carried tools-js include");
    let narrow = scan.replace(&family, "  \"tools/**/*.js\",\n");
    let module = render(&["tools/chromium/run.mjs".to_string()], &[], &narrow);
    assert!(!strings(&module, "scan", "include").contains(&"tools/**/*.js".to_string()));
    let script = render(&["tools/chromium/run.js".to_string()], &[], &narrow);
    assert!(strings(&script, "scan", "include").contains(&"tools/**/*.js".to_string()));
}

#[test]
fn resources() {
    let data = render(
        &["tools/spec/resources/harness.js".to_string()],
        &[],
        &carried("atoms/scan.toml"),
    );
    assert!(earned(&data, "tools"), "{data}");
    assert!(strings(&data, "scan", "exclude").contains(&"tools/*/resources/**".to_string()));
    let script = render(
        &["tools/chromium/cards.js".to_string()],
        &[],
        &carried("atoms/scan.toml"),
    );
    assert!(earned(&script, "tools"), "{script}");
    assert!(strings(&script, "module", "roots").contains(&"tools/*".to_string()));
    assert!(strings(&script, "scan", "exclude").contains(&"tools/*/resources/**".to_string()));
}
