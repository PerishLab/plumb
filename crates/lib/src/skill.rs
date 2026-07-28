mod agent;
mod fetch;
mod place;
mod state;
mod survey;

pub use agent::Seat;
pub use fetch::stamp;
pub use state::Record;
pub use survey::{Action, Report, Standing, Status, Target};

use state::Ledger;
use std::path::PathBuf;

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
    Version(String),
    Bare,
    Named(PathBuf),
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
            Self::Absent => write!(f, "release carries no skill artifact"),
            Self::Loose => write!(f, "release names no digest for the skill artifact"),
            Self::Shape(name) => write!(f, "unexpected artifact {name}"),
            Self::Channel(channel) => write!(f, "invalid release channel: {channel}"),
            Self::Floating(channel) => {
                write!(f, "non-stable channel {channel} requires an exact version")
            }
            Self::Version(version) => write!(f, "invalid release version: {version}"),
            Self::Bare => write!(f, "no agent skill directory was found"),
            Self::Named(path) => write!(f, "path must end with the skill name: {}", path.display()),
        }
    }
}

impl std::error::Error for Error {}

impl Kit {
    pub fn install(&self, ask: &Ask) -> Result<Done, Error> {
        let seats = match &ask.path {
            Some(path) => self.chosen(path)?,
            None => agent::seats(&self.home, &self.name),
        };
        if seats.is_empty() {
            return Err(Error::Bare);
        }
        self.lay(ask, seats)
    }

    pub fn upgrade(&self, ask: &Ask) -> Result<Done, Error> {
        let grant = fetch::resolve(&self.url, &ask.channel, ask.version.as_deref())?;
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
        let bytes = changes.then(|| fetch::take(&grant)).transpose()?;
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
        let grant = fetch::resolve(&self.url, &ask.channel, ask.version.as_deref())?;
        let ledger = state::read(&self.state)?;
        survey::inspect(self, ask, &grant, &ledger)
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

    fn lay(&self, ask: &Ask, seats: Vec<Seat>) -> Result<Done, Error> {
        let grant = fetch::resolve(&self.url, &ask.channel, ask.version.as_deref())?;
        let bytes = fetch::take(&grant)?;
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
