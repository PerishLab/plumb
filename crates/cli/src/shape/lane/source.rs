use crate::command::depot::held;
use std::collections::BTreeMap;
use std::sync::LazyLock;

pub const RELEASED: [(&str, &str); 2] = [
    ("exact.release.yml", "assets/release/exact.yml.in"),
    ("stable.release.yml", "assets/release/stable.yml.in"),
];

pub const FACTORY: [(&str, &str); 16] = [
    (
        "assets/depot/lane.yml.in",
        plumb::seat::resource!("assets/depot/lane.yml.in"),
    ),
    (
        "assets/guard/ask.yml.in",
        plumb::seat::resource!("assets/guard/ask.yml.in"),
    ),
    (
        "assets/guard/lane.yml.in",
        plumb::seat::resource!("assets/guard/lane.yml.in"),
    ),
    (
        "assets/guard/packages.yml.in",
        plumb::seat::resource!("assets/guard/packages.yml.in"),
    ),
    (
        "assets/guard/proof.yml.in",
        plumb::seat::resource!("assets/guard/proof.yml.in"),
    ),
    (
        "assets/guard/tool.yml.in",
        plumb::seat::resource!("assets/guard/tool.yml.in"),
    ),
    (
        "assets/manager/unix.sh.in",
        plumb::seat::resource!("assets/manager/unix.sh.in"),
    ),
    (
        "assets/manager/windows.ps1.in",
        plumb::seat::resource!("assets/manager/windows.ps1.in"),
    ),
    (
        "assets/release/exact.yml.in",
        plumb::seat::resource!("assets/release/exact.yml.in"),
    ),
    (
        "assets/release/stable.yml.in",
        plumb::seat::resource!("assets/release/stable.yml.in"),
    ),
    (
        "assets/ship/binary.yml.in",
        plumb::seat::resource!("assets/ship/binary.yml.in"),
    ),
    (
        "assets/ship/capsule.yml.in",
        plumb::seat::resource!("assets/ship/capsule.yml.in"),
    ),
    (
        "assets/ship/install.yml.in",
        plumb::seat::resource!("assets/ship/install.yml.in"),
    ),
    (
        "assets/ship/lane.yml.in",
        plumb::seat::resource!("assets/ship/lane.yml.in"),
    ),
    (
        "assets/ship/project.yml.in",
        plumb::seat::resource!("assets/ship/project.yml.in"),
    ),
    (
        "assets/ship/windows.yml.in",
        plumb::seat::resource!("assets/ship/windows.yml.in"),
    ),
];

static SOURCE: LazyLock<Result<BTreeMap<&'static str, String>, String>> = LazyLock::new(read);

fn read() -> Result<BTreeMap<&'static str, String>, String> {
    let seat = held();
    let mut listed = BTreeMap::new();
    for (path, factory) in FACTORY {
        listed.insert(path, seat.read(path, factory)?);
    }
    Ok(listed)
}

pub fn text(path: &str) -> Result<&'static str, String> {
    let listed = SOURCE.as_ref().map_err(Clone::clone)?;
    listed
        .get(path)
        .map(String::as_str)
        .ok_or_else(|| format!("no template is carried at {path}"))
}

pub fn filled(path: &str, vars: &BTreeMap<&str, String>) -> Result<String, String> {
    plumb::fill::actions(text(path)?, vars).map_err(|error| error.to_string())
}

pub fn plain(path: &str) -> Result<String, String> {
    filled(path, &BTreeMap::new())
}
