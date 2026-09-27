use std::process::Command;

#[test]
fn commands() {
    let binary = env!("CARGO_BIN_EXE_plumb");
    let help = Command::new(binary)
        .arg("--help")
        .output()
        .expect("top-level help");
    assert!(help.status.success());
    let help = String::from_utf8_lossy(&help.stdout);
    assert!(help.contains("land"));
    assert!(
        !help
            .lines()
            .any(|line| line.trim_start().starts_with("plan"))
    );

    let removed = Command::new(binary)
        .args(["plan", "delivery", "--help"])
        .output()
        .expect("removed plan command");
    assert!(!removed.status.success());

    let land = Command::new(binary)
        .args(["land", "--help"])
        .output()
        .expect("repository landing help");
    assert!(land.status.success());
    let land = String::from_utf8_lossy(&land.stdout);
    assert!(land.contains("--title"));
    assert!(land.contains("--body"));
    assert!(!land.contains("--plan"));
}
