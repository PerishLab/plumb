use super::{Ask, Error, Kit, Record, fetch, place, state};
use semver::Version;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize)]
pub struct Target {
    pub version: String,
    pub url: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Report {
    pub channel: String,
    pub explicit: bool,
    pub target: Target,
    pub seats: Vec<Status>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Status {
    pub agent: String,
    pub path: PathBuf,
    pub installed: String,
    pub state: Standing,
    pub action: Action,
    pub note: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Standing {
    Current,
    UpdateAvailable,
    Ahead,
    Missing,
    OwnershipMismatch,
    MetadataDrift,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    None,
    Upgrade,
    Rollback,
    Restore,
    Refuse,
}

impl std::fmt::Display for Standing {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(match self {
            Self::Current => "current",
            Self::UpdateAvailable => "update_available",
            Self::Ahead => "ahead",
            Self::Missing => "missing",
            Self::OwnershipMismatch => "ownership_mismatch",
            Self::MetadataDrift => "metadata_drift",
        })
    }
}

impl std::fmt::Display for Action {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(match self {
            Self::None => "none",
            Self::Upgrade => "upgrade",
            Self::Rollback => "rollback",
            Self::Restore => "restore",
            Self::Refuse => "refuse",
        })
    }
}

pub(super) fn inspect(
    kit: &Kit,
    ask: &Ask,
    grant: &fetch::Grant,
    ledger: &state::Ledger,
) -> Result<Report, Error> {
    let target = version(&grant.version)?;
    let explicit = ask.version.is_some();
    let seats = ledger
        .records
        .iter()
        .map(|record| inspect_seat(kit, record, grant, &target, explicit))
        .collect::<Result<_, _>>()?;
    Ok(Report {
        channel: ask.channel.clone(),
        explicit,
        target: Target {
            version: grant.version.clone(),
            url: grant.url.clone(),
            sha256: grant.sha.clone(),
        },
        seats,
    })
}

fn inspect_seat(
    kit: &Kit,
    record: &Record,
    grant: &fetch::Grant,
    target: &Version,
    explicit: bool,
) -> Result<Status, Error> {
    if !record.path.exists() {
        return Ok(status(
            record,
            Standing::Missing,
            Action::Restore,
            "managed path is absent; restore the selected release",
        ));
    }
    let Some(marker) = place::marker(&record.path, &kit.name) else {
        return Ok(status(
            record,
            Standing::OwnershipMismatch,
            Action::Refuse,
            "unmanaged path; matching ownership marker is absent",
        ));
    };
    if marker.version != record.version {
        return Ok(status(
            record,
            Standing::MetadataDrift,
            Action::Refuse,
            "ledger and ownership marker name different installed versions",
        ));
    }
    let installed = version(&record.version)?;
    let (standing, action, note) = match installed.cmp(target) {
        std::cmp::Ordering::Equal if record.sha == grant.sha => (
            Standing::Current,
            Action::None,
            "installed release and artifact digest are current",
        ),
        std::cmp::Ordering::Equal => (
            Standing::MetadataDrift,
            Action::Refuse,
            "the selected immutable version names a different artifact digest",
        ),
        std::cmp::Ordering::Less => (
            Standing::UpdateAvailable,
            Action::Upgrade,
            "a newer selected release is available",
        ),
        std::cmp::Ordering::Greater if explicit => (
            Standing::Ahead,
            Action::Rollback,
            "the explicit version selects a rollback",
        ),
        std::cmp::Ordering::Greater => (
            Standing::Ahead,
            Action::Refuse,
            "channel metadata is older than the installed release",
        ),
    };
    Ok(status(record, standing, action, note))
}

fn version(raw: &str) -> Result<Version, Error> {
    Version::parse(raw.trim().trim_start_matches('v')).map_err(|_| Error::Version(raw.to_string()))
}

fn status(record: &Record, state: Standing, action: Action, note: &str) -> Status {
    Status {
        agent: record.agent.clone(),
        path: record.path.clone(),
        installed: record.version.clone(),
        state,
        action,
        note: note.to_string(),
    }
}
