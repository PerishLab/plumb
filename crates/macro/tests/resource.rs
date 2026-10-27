#[test]
fn carried() {
    assert_eq!(plumb_macro::resource!("tests/held/kept.txt"), "held\n");
}

#[test]
fn binary() {
    const HELD: &[u8; 5] = plumb_macro::bytes!("tests/held/kept.bin");
    assert_eq!(HELD, &[0, 255, 128, 13, 10]);
    assert_eq!(plumb_macro::bytes!("tests/held/kept.txt"), b"held\n");
}
