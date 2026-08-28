use std::process::Command;

#[test]
fn known() {
    let seat = super::fixture();
    let lanes = seat.path().join(".forgejo/workflows");
    std::fs::create_dir_all(&lanes).expect("workflow seat should be made");
    for atom in ["guard.atom", "plan.atom"] {
        std::fs::write(lanes.join(format!("{atom}.yml")), "")
            .expect("public atom should be written");
    }

    let depot = super::super::support::depot(&[]);
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["doctor", seat.path().to_str().expect("path should be utf8")])
        .env_remove("PLUMB_RELEASE_VERSION")
        .env("PLUMB_DEPOT_SEAT", depot.path())
        .output()
        .expect("plumb should run");
    let out = String::from_utf8_lossy(&output.stdout);
    assert!(out.contains("workflow guard.atom has no shadow"), "{out}");
    assert!(!out.contains("workflow plan.atom has no shadow"), "{out}");
}
