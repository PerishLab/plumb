const SECRET: &str = "RELEASE_REGISTRY_TOKEN";

#[allow(dead_code)]
#[path = "../../src/command/release/authority/profile.rs"]
mod profile;

#[test]
#[cfg(unix)]
fn state() {
    use std::os::unix::fs::PermissionsExt as _;

    let fixture = tempfile::tempdir().expect("fixture");
    let path = fixture.path().join("registry.env");
    let seat = profile::Seat::new(&path);
    let held = profile::Escrow {
        id: 42,
        name: "plumb-release-registry-v1".into(),
        last: "12345678".into(),
        secret: "secret-value-12345678".into(),
    };
    seat.write(&held).expect("write escrow");
    seat.binding().write(42).expect("write binding");
    assert_eq!(
        std::fs::metadata(&path)
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    let loaded = seat.load().expect("load").expect("present");
    assert_eq!(loaded.view(), held.view());
    assert_eq!(seat.binding().load().expect("binding"), Some(42));
    seat.retire().expect("retire");
    assert!(seat.load().expect("absent").is_none());
    assert!(seat.binding().load().expect("unbound").is_none());
}
