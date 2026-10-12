#[test]
fn construction() {
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(concurrent);
        }
    });
}

fn concurrent() {
    for _ in 0..128 {
        let root = tempfile::tempdir().unwrap();
        super::tool(root.path(), "#!/bin/sh\nprintf '%s' \"$PATH\"\n");
        let held = super::execution(root.path(), &[root.path()]).unwrap();
        let output = held.command("probe").unwrap().output().unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, root.path().as_os_str().as_encoded_bytes());
    }
}
