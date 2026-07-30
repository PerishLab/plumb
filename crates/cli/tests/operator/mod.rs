use std::process::Command;

#[cfg(unix)]
mod fixture;
#[cfg(unix)]
mod release;

fn run(root: &std::path::Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["doctor", root.to_str().expect("path should be utf8")])
        .output()
        .expect("plumb should run");
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn actions() {
    let dir = std::env::temp_dir().join("plumb-actions");
    std::fs::create_dir_all(dir.join("setup-tool")).expect("fixture should be made");
    std::fs::write(dir.join("setup-tool/action.yml"), "name: setup\n")
        .expect("action should be written");
    let held = run(&dir);
    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
    assert!(
        !held.contains("directory setup-tool has no shadow"),
        "{held}"
    );
}

#[test]
fn adoption() {
    let dir = std::env::temp_dir().join("plumb-adoption");
    std::fs::create_dir_all(dir.join(".runseal/wrappers")).expect("fixture should be made");
    for name in ["guard", "init", "land"] {
        std::fs::write(dir.join(format!(".runseal/wrappers/{name}.ts")), "")
            .expect("wrapper should be written");
    }
    let bare = run(&dir);
    assert!(bare.contains("guard does not run plumb doctor"), "{bare}");
    assert!(
        bare.contains("guard does not run ectropy explicitly"),
        "{bare}"
    );
    assert!(bare.contains("init does not require plumb"), "{bare}");

    std::fs::write(
        dir.join(".runseal/wrappers/guard.ts"),
        "await bin(\"plumb\").run([\"doctor\", \".\"]);\nawait bin(\"ectropy\").run([\".\"]);\n",
    )
    .expect("guard should be written");
    std::fs::write(
        dir.join(".runseal/wrappers/init.ts"),
        "await init({ tools: [\"plumb\"] });\n",
    )
    .expect("init should be written");
    let held = run(&dir);
    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
    assert!(!held.contains("guard does not run plumb doctor"), "{held}");
    assert!(
        !held.contains("guard does not run ectropy explicitly"),
        "{held}"
    );
    assert!(!held.contains("init does not require plumb"), "{held}");
}

#[test]
fn forbidden() {
    let dir = std::env::temp_dir().join("plumb-operator-test");
    std::fs::create_dir_all(dir.join(".runseal/lib")).expect("fixture should be made");
    for name in [
        "control.test.ts",
        "control_test.ts",
        "control.test.tsx",
        "control_test.tsx",
    ] {
        std::fs::write(dir.join(format!(".runseal/lib/{name}")), "")
            .expect("test should be written");
    }
    let held = run(&dir);
    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
    for name in [
        "control.test.ts",
        "control_test.ts",
        "control.test.tsx",
        "control_test.tsx",
    ] {
        let path = std::path::Path::new(".runseal").join("lib").join(name);
        assert!(
            held.contains(&format!(
                "{} is a .runseal test; tested logic belongs in sealkit",
                path.display()
            )),
            "{held}"
        );
    }
}

#[test]
fn obsolete() {
    let dir = std::env::temp_dir().join("plumb-obsolete");
    std::fs::create_dir_all(dir.join(".runseal/wrappers")).expect("fixture should be made");
    for name in ["guard", "init", "land"] {
        std::fs::write(dir.join(format!(".runseal/wrappers/{name}.ts")), "")
            .expect("wrapper should be written");
    }
    std::fs::write(
        dir.join(".runseal/wrappers/guard.ts"),
        "await bin(\"plumb\").run([\"doctor\", \".\"]);\nawait bin(\"ectropy\").run([\"--strict\", \".\"]);\n",
    )
    .expect("guard should be written");
    let held = run(&dir);
    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
    assert!(
        held.contains("guard uses an obsolete ectropy mode"),
        "{held}"
    );
}
