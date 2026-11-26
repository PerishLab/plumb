#[plumb_macro::prefix]
struct Defaulted;

#[plumb_macro::prefix("FOLIO")]
struct Named;

#[plumb_macro::prefix("APP_2")]
struct Scoped;

#[test]
fn bare() {
    assert_eq!(Defaulted::prefix(), "PLUMB_MACRO");
}

#[test]
fn explicit() {
    assert_eq!(Named::prefix(), "FOLIO");
    assert_eq!(Scoped::prefix(), "APP_2");
}
