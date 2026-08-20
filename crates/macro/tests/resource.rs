#[test]
fn carried() {
    assert_eq!(plumb_macro::resource!("tests/held/kept.txt"), "held\n");
}
