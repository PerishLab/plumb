use super::{public, wire};

#[test]
fn stable() {
    let bytes = public(&wire(&[7; 32]));
    let first = plumb::instance::parse(&bytes).unwrap();
    assert_eq!(first, plumb::instance::parse(&bytes).unwrap());
    assert!(first.starts_with("ssh-ed25519:SHA256:"));
    assert_ne!(
        first,
        plumb::instance::parse(&public(&wire(&[8; 32]))).unwrap()
    );
}

#[test]
fn comments() {
    let bytes = public(&wire(&[7; 32]));
    let text = String::from_utf8(bytes.clone()).unwrap();
    let other = format!(
        " \n\t{}\r\n\n",
        text.trim().replace("fixture", "other comment")
    );
    assert_eq!(
        plumb::instance::parse(&bytes).unwrap(),
        plumb::instance::parse(other.as_bytes()).unwrap()
    );
}

#[test]
fn duplicate() {
    let mut bytes = public(&wire(&[7; 32]));
    bytes.extend(bytes.clone());
    assert!(plumb::instance::parse(&bytes).is_err());
}

#[test]
fn invalid() {
    for bytes in [
        b"".as_slice(),
        b" \n\t",
        b"ssh-ed25519",
        b"ssh-ed25519 ????",
        b"ssh-rsa AAAA",
        b"\xff",
    ] {
        assert!(plumb::instance::parse(bytes).is_err());
    }
}

#[test]
fn truncated() {
    let complete = wire(&[7; 32]);
    for size in 0..complete.len() {
        assert!(plumb::instance::parse(&public(&complete[..size])).is_err());
    }
}

#[test]
fn mismatch() {
    let mut bytes = wire(&[7; 32]);
    bytes[4] = b'x';
    assert!(plumb::instance::parse(&public(&bytes)).is_err());
    assert!(plumb::instance::parse(&public(&wire(&[7; 31]))).is_err());
    let mut trailing = wire(&[7; 32]);
    trailing.push(0);
    assert!(plumb::instance::parse(&public(&trailing)).is_err());
}

#[test]
fn enormous() {
    let mut bytes = wire(&[7; 32]);
    bytes[..4].copy_from_slice(&u32::MAX.to_be_bytes());
    assert!(plumb::instance::parse(&public(&bytes)).is_err());
}
