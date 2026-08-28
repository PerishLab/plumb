use super::super::depot::held;
use std::collections::BTreeMap;
use std::sync::LazyLock;

pub const FACTORY: [(&str, &str); 19] = [
    (
        "assets/depot/lane.yml.in",
        plumb::seat::resource!("assets/depot/lane.yml.in"),
    ),
    (
        "assets/guard/ask.yml.in",
        plumb::seat::resource!("assets/guard/ask.yml.in"),
    ),
    (
        "assets/guard/atom.yml.in",
        plumb::seat::resource!("assets/guard/atom.yml.in"),
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
        "assets/guard/sync.yml.in",
        plumb::seat::resource!("assets/guard/sync.yml.in"),
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
        "assets/ship/source.yml.in",
        plumb::seat::resource!("assets/ship/source.yml.in"),
    ),
    (
        "assets/ship/windows.yml.in",
        plumb::seat::resource!("assets/ship/windows.yml.in"),
    ),
];

static CARRIED: LazyLock<Result<bool, String>> = LazyLock::new(carried);

#[derive(Clone, Copy)]
pub enum Store {
    Depot,
    Factory,
}

fn carried() -> Result<bool, String> {
    let seat = held();
    let lane = FACTORY
        .iter()
        .find(|(path, _)| *path == "assets/guard/lane.yml.in")
        .expect("the factory must carry its guard lane");
    let schema = seat.read(lane.0, lane.1)?;
    Ok(schema.contains("depot-sync/v2"))
}

pub fn text(path: &str) -> Result<String, String> {
    Store::Depot.text(path)
}

pub fn filled(path: &str, vars: &BTreeMap<&str, String>) -> Result<String, String> {
    Store::Depot.filled(path, vars)
}

impl Store {
    pub fn text(self, path: &str) -> Result<String, String> {
        let factory = FACTORY
            .iter()
            .find(|(held, _)| *held == path)
            .map(|(_, factory)| *factory)
            .ok_or_else(|| format!("no template is carried at {path}"))?;
        if matches!(self, Self::Depot) && *CARRIED.as_ref().map_err(Clone::clone)? {
            held().read(path, factory)
        } else {
            Ok(factory.to_string())
        }
    }

    pub fn filled(self, path: &str, vars: &BTreeMap<&str, String>) -> Result<String, String> {
        plumb::fill::actions(&self.text(path)?, vars).map_err(|error| error.to_string())
    }

    pub fn matrixed(self, install: &str, vars: &BTreeMap<&str, String>) -> Result<String, String> {
        let guarded = install.replacen(
            "        run: |",
            "        if: runner.os != 'Windows'\n        run: |",
            1,
        );
        Ok(format!(
            "{guarded}\n{}",
            self.filled("assets/ship/windows.yml.in", vars)?
        ))
    }
}
