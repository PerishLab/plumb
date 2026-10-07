use clap::Parser as _;

#[test]
fn consign() {
    let hint = super::consign("v1.0.0");
    crate::Cli::try_parse_from(hint.split_whitespace()).unwrap_or_else(|error| {
        panic!("the changelog hint names no Plumb command: {hint}\n{error}")
    });
}
