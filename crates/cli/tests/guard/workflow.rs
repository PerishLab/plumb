use std::process::Command;

#[test]
fn retired() {
    for verb in ["plan", "record", "ask", "hash", "lock", "status"] {
        let seat = tempfile::tempdir().unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
            .current_dir(seat.path())
            .env("PLUMB_HOME", seat.path())
            .args(["workflow", verb])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("unrecognized subcommand 'workflow'")
        );
    }
}
