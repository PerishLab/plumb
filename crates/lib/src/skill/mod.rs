mod agent;
mod fetch;
mod place;
mod source;
mod state;
mod survey;

pub use agent::Seat;
pub use fetch::stamp;
pub use source::Depot;
use state::Ledger;
pub use state::Record;
use std::path::PathBuf;
pub use survey::{Action, Report, Standing, Status, Target};

pub struct Kit {
    pub name: String,
    pub home: PathBuf,
    pub state: PathBuf,
    pub url: String,
}

#[derive(Default)]
pub struct Ask {
    pub channel: String,
    pub version: Option<String>,
    pub path: Option<PathBuf>,
    pub force: bool,
}

pub struct Skip {
    pub path: PathBuf,
    pub note: String,
}

#[derive(Default)]
pub struct Done {
    pub kept: Vec<Seat>,
    pub same: Vec<Seat>,
    pub left: Vec<Skip>,
}

#[derive(Debug)]
pub enum Error {
    Read(PathBuf, String),
    Write(PathBuf, String),
    Parse(String),
    Schema(u32),
    Fetch(String, String),
    Digest(String, String),
    Unpack(String),
    Absent,
    Loose,
    Shape(String),
    Channel(String),
    Floating(String),
    Managed(String),
    Stage,
    Version(String),
    Bare,
    Named(PathBuf),
    Occupied(PathBuf),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read(path, why) => write!(f, "cannot read {}: {why}", path.display()),
            Self::Write(path, why) => write!(f, "cannot write {}: {why}", path.display()),
            Self::Parse(why) => write!(f, "malformed json: {why}"),
            Self::Schema(seen) => write!(f, "unknown state schema {seen}"),
            Self::Fetch(url, why) => write!(f, "cannot fetch {url}: {why}"),
            Self::Digest(want, seen) => write!(f, "digest mismatch: want {want} got {seen}"),
            Self::Unpack(why) => write!(f, "cannot unpack: {why}"),
            Self::Absent => write!(f, "selected version carries no skill"),
            Self::Loose => write!(f, "release names no digest for the skill artifact"),
            Self::Shape(name) => write!(f, "unexpected artifact {name}"),
            Self::Channel(channel) => write!(f, "invalid release channel: {channel}"),
            Self::Floating(channel) => {
                write!(f, "non-stable channel {channel} requires an exact version")
            }
            Self::Managed(channel) => {
                write!(f, "managed skills only admit stable, not {channel}")
            }
            Self::Stage => write!(f, "stable belongs in managed skill seats, not staging"),
            Self::Version(version) => write!(f, "invalid release version: {version}"),
            Self::Bare => write!(f, "no agent skill directory was found"),
            Self::Named(path) => write!(f, "path must end with the skill name: {}", path.display()),
            Self::Occupied(path) => {
                write!(f, "staging path must not exist: {}", path.display())
            }
        }
    }
}

impl std::error::Error for Error {}

impl Kit {
    pub fn install(&self, ask: &Ask) -> Result<Done, Error> {
        self.installing(&source::Release(&self.url), ask)
    }

    fn installing(&self, source: &impl source::Source, ask: &Ask) -> Result<Done, Error> {
        managed(ask)?;
        let seats = match &ask.path {
            Some(path) => self.chosen(path)?,
            None => agent::seats(&self.home, &self.name),
        };
        if seats.is_empty() {
            return Err(Error::Bare);
        }
        self.lay(source, ask, seats)
    }

    pub fn upgrade(&self, ask: &Ask) -> Result<Done, Error> {
        self.upgrading(&source::Release(&self.url), ask)
    }

    fn upgrading(&self, source: &impl source::Source, ask: &Ask) -> Result<Done, Error> {
        managed(ask)?;
        let grant = source.resolve(ask)?;
        let ledger = state::read(&self.state)?;
        if ledger.records.is_empty() {
            return Err(Error::Bare);
        }
        let report = survey::inspect(self, ask, &grant, &ledger)?;
        let changes = report.seats.iter().any(|status| {
            matches!(
                status.action,
                Action::Upgrade | Action::Rollback | Action::Restore
            )
        });
        let bytes = changes
            .then(|| fetch::take(&grant, &self.name))
            .transpose()?;
        let mut ledger = ledger;
        let mut done = Done::default();
        for status in report.seats {
            let seat = Seat {
                agent: status.agent,
                path: status.path,
            };
            match status.action {
                Action::None => done.same.push(seat),
                Action::Refuse => done.left.push(Skip {
                    path: seat.path,
                    note: status.note,
                }),
                Action::Upgrade | Action::Rollback | Action::Restore => {
                    let bytes = bytes
                        .as_deref()
                        .expect("a changing report fetched its artifact");
                    place::place(self, &seat, bytes, &grant.version)?;
                    state::keep(&mut ledger, mark(&seat, &grant));
                    done.kept.push(seat);
                }
            }
        }
        if !done.kept.is_empty() {
            state::write(&self.state, &ledger)?;
        }
        Ok(done)
    }

    pub fn status(&self, ask: &Ask) -> Result<Report, Error> {
        self.inspecting(&source::Release(&self.url), ask)
    }

    fn inspecting(&self, source: &impl source::Source, ask: &Ask) -> Result<Report, Error> {
        managed(ask)?;
        let grant = source.resolve(ask)?;
        let ledger = state::read(&self.state)?;
        survey::inspect(self, ask, &grant, &ledger)
    }

    pub fn stage(&self, ask: &Ask) -> Result<Done, Error> {
        self.staging(&source::Release(&self.url), ask)
    }

    fn staging(&self, source: &impl source::Source, ask: &Ask) -> Result<Done, Error> {
        if ask.channel.trim() == "stable" {
            return Err(Error::Stage);
        }
        let path = ask.path.as_ref().ok_or(Error::Bare)?;
        let seats = self.chosen(path)?;
        if path.exists() {
            return Err(Error::Occupied(path.clone()));
        }
        let grant = source.resolve(ask)?;
        let bytes = fetch::take(&grant, &self.name)?;
        let seat = seats.into_iter().next().expect("a chosen path is one seat");
        place::stage(self, &seat, &bytes, &grant.version)?;
        Ok(Done {
            kept: vec![seat],
            ..Done::default()
        })
    }

    pub fn list(&self) -> Result<Vec<Record>, Error> {
        Ok(state::read(&self.state)?.records)
    }

    pub fn uninstall(&self) -> Result<Done, Error> {
        let mut ledger = state::read(&self.state)?;
        let mut done = Done::default();
        for seat in ledger.records.iter().map(worn).collect::<Vec<_>>() {
            if seat.path.exists() && !place::owned(&seat.path, &self.name) {
                done.left.push(guest(&seat.path));
                continue;
            }
            place::erase(&seat.path)?;
            ledger.records.retain(|held| held.path != seat.path);
            done.kept.push(seat);
        }
        state::write(&self.state, &ledger)?;
        Ok(done)
    }

    fn lay(
        &self,
        source: &impl source::Source,
        ask: &Ask,
        seats: Vec<Seat>,
    ) -> Result<Done, Error> {
        let grant = source.resolve(ask)?;
        let bytes = fetch::take(&grant, &self.name)?;
        let mut ledger = state::read(&self.state)?;
        let mut done = Done::default();
        for seat in seats {
            match self.ready(&ledger, &seat, ask.force) {
                Some(note) => done.left.push(Skip {
                    path: seat.path.clone(),
                    note,
                }),
                None => {
                    place::place(self, &seat, &bytes, &grant.version)?;
                    state::keep(&mut ledger, mark(&seat, &grant));
                    done.kept.push(seat);
                }
            }
        }
        state::write(&self.state, &ledger)?;
        Ok(done)
    }

    fn ready(&self, ledger: &Ledger, seat: &Seat, force: bool) -> Option<String> {
        if !seat.path.exists() {
            return None;
        }
        if !place::owned(&seat.path, &self.name) || !state::held(ledger, &seat.path) {
            return Some("unmanaged path; refusing to replace it".to_string());
        }
        if force {
            return None;
        }
        Some("already installed; pass force to replace it".to_string())
    }

    fn chosen(&self, path: &std::path::Path) -> Result<Vec<Seat>, Error> {
        if !agent::named(path, &self.name) {
            return Err(Error::Named(path.to_path_buf()));
        }
        Ok(vec![Seat {
            agent: "chosen".to_string(),
            path: path.to_path_buf(),
        }])
    }
}

fn managed(ask: &Ask) -> Result<(), Error> {
    let channel = ask.channel.trim();
    if channel != "stable" {
        return Err(Error::Managed(channel.to_string()));
    }
    Ok(())
}

fn worn(record: &Record) -> Seat {
    Seat {
        agent: record.agent.clone(),
        path: record.path.clone(),
    }
}

fn mark(seat: &Seat, grant: &fetch::Grant) -> Record {
    Record {
        agent: seat.agent.clone(),
        path: seat.path.clone(),
        version: grant.version.clone(),
        url: grant.url.clone(),
        sha: grant.sha.clone(),
    }
}

fn guest(path: &std::path::Path) -> Skip {
    Skip {
        path: path.to_path_buf(),
        note: "unmanaged path; refusing to remove it".to_string(),
    }
}
