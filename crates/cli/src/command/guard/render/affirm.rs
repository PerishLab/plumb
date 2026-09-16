use crate::judge::structure::layout::{Faces, authority, rule};
use crate::shape::layout::{Declared, Held};
use plumb::snapshot::Snapshot;
use std::path::Path;

pub const SEAT: &str = ".plumb/affirmed.toml";

#[path = "candidate.rs"]
pub(crate) mod candidate;
#[path = "receipt.rs"]
mod receipt;

#[derive(clap::Args)]
pub struct Request {
    #[command(flatten)]
    target: crate::Root,
    #[arg(long)]
    write: bool,
    #[arg(
        long,
        value_name = "PATH",
        help = "Review candidate configuration media; --write records its document confirmation"
    )]
    pub configuration: Option<std::path::PathBuf>,
}

impl Request {
    pub fn run(self) -> i32 {
        match self.configuration {
            Some(media) => candidate::Review(Path::new(&self.target.root)).run(&media, self.write),
            None => super::Seat::new(self.target.root.into()).affirm(self.write),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Owed {
    pub target: String,
    pub rule: String,
    pub authority: String,
}

pub fn owed(root: &Path) -> Result<Vec<Owed>, String> {
    let snapshot = Snapshot::read(root).map_err(|error| error.to_string())?;
    let Held::Stated(declared) = crate::shape::layout::stated(root) else {
        return Err("no layout is declared".to_string());
    };
    let mut found = Vec::new();
    for seat in declared.seats.iter().filter(|seat| !seat.retired) {
        for held in &seat.rule {
            let Ok(member) = rule::parse(held).and_then(|held| rule::member(&held)) else {
                continue;
            };
            let (Some(container), Some(leaf)) = (seat.container(), member.leaf.as_deref()) else {
                continue;
            };
            let targets = Faces(&snapshot)
                .members(container)
                .into_iter()
                .map(|name| format!("{name}/{leaf}"))
                .collect::<Vec<_>>();
            found.extend(Stamp(&snapshot, &declared, &member).owed(held, &targets));
        }
    }
    for group in declared.groups.iter().filter(|group| !group.retired) {
        for held in &group.rule {
            let Ok(member) = rule::parse(held).and_then(|held| rule::member(&held)) else {
                continue;
            };
            found.extend(Stamp(&snapshot, &declared, &member).owed(held, &group.names));
        }
    }
    Ok(found)
}

struct Stamp<'a>(&'a Snapshot, &'a Declared, &'a rule::Member);

impl Stamp<'_> {
    fn owed(&self, rule: &str, targets: &[String]) -> Vec<Owed> {
        if self.2.affirms.is_empty() {
            return Vec::new();
        }
        let authority = authority(&Faces(self.0).taken(self.1, &self.2.affirms));
        targets
            .iter()
            .map(|target| Owed {
                target: target.clone(),
                rule: rule.to_string(),
                authority: authority.clone(),
            })
            .collect()
    }
}

pub fn write(root: &Path, owed: &[Owed]) -> Result<(), String> {
    #[derive(serde::Serialize)]
    struct Document<'a> {
        record: &'a [Owed],
    }
    let body = toml::to_string(&Document { record: owed }).map_err(|error| error.to_string())?;
    let seat = root.join(SEAT);
    if let Some(base) = seat.parent() {
        std::fs::create_dir_all(base).map_err(|error| error.to_string())?;
    }
    std::fs::write(&seat, body).map_err(|error| error.to_string())
}
