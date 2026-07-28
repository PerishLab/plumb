use std::process::Command;

fn run(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(args)
        .output()
        .expect("plumb should run");
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn sites() {
    let dir = std::env::temp_dir().join("plumb-siteless");
    std::fs::create_dir_all(dir.join("apps/web")).expect("fixture should be made");
    std::fs::create_dir_all(dir.join(".runseal/wrappers")).expect("fixture should be made");
    let bare = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    assert!(!bare.contains("declares a site"), "{bare}");

    std::fs::write(dir.join("apps/web/wrangler.jsonc"), "{}").expect("config should be written");
    let held = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    assert!(
        held.contains("web declares a site without a ship wrapper"),
        "{held}"
    );
    assert!(
        held.contains("web declares a site without a deploy lane"),
        "{held}"
    );

    std::fs::write(dir.join(".runseal/wrappers/ship.ts"), "").expect("wrapper should be written");
    std::fs::create_dir_all(dir.join(".forgejo/workflows")).expect("fixture should be made");
    std::fs::write(dir.join(".forgejo/workflows/deploy.yml"), "").expect("lane should be written");
    let paired = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
    assert!(!paired.contains("declares a site"), "{paired}");
    assert!(paired.contains("sites     web"), "{paired}");
}
