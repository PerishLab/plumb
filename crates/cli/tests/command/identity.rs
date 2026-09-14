use std::{fs, process::Command};

#[test]
#[cfg(target_os = "linux")]
fn executable() {
    let source = env!("CARGO_BIN_EXE_plumb");
    let original = fs::read(source).unwrap();
    let origin = plumb::identity::inspect(&original).unwrap().0;
    if option_env!("PLUMB_BUILD_CHANNEL") == Some("unbound") {
        let result = Command::new(source).arg("--version").output().unwrap();
        assert!(result.status.success());
        assert_eq!(
            String::from_utf8(result.stdout).unwrap().trim(),
            "plumb unbound"
        );
        let result = Command::new(source)
            .args(["skill", "list"])
            .output()
            .unwrap();
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("unbound build"));
    }
    let directory = tempfile::tempdir().unwrap();
    for marker in ["v0.37.39-beta.2", "v0.37.39"] {
        let identity = plumb::identity::Binding {
            product: "plumb".into(),
            marker: marker.into(),
            digest: "2".repeat(64),
            commit: "3".repeat(40),
            workload: "4".repeat(64),
        };
        let bound = plumb::identity::bind(&original, &identity).unwrap();
        let inspected = plumb::identity::inspect(&bound).unwrap();
        assert_eq!(inspected, (origin.clone(), Some(identity)));
        let path = directory.path().join(marker);
        fs::write(&path, bound).unwrap();
        fs::set_permissions(&path, fs::metadata(source).unwrap().permissions()).unwrap();
        let result = Command::new(&path).arg("--version").output().unwrap();
        assert!(result.status.success());
        assert_eq!(
            String::from_utf8(result.stdout).unwrap().trim(),
            format!("plumb {marker}")
        );
        let result = Command::new(&path)
            .args(["skill", "--help"])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    assert_eq!(fs::read(source).unwrap(), original);
}
