mod observe;
pub(super) mod retire;

use super::{Input, Provider, Seat, State, provider::Intent, repository, work};
use observe::Recovery;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    schema: String,
    state: String,
    local: String,
    head: String,
    merge: String,
    guard: String,
    candidate: plumb::guard::Descriptor,
    history: Vec<Intent>,
}

pub(crate) fn command(input: Input, number: u64) -> i32 {
    let recovered = repository(&input.root).and_then(|repository| {
        let provider = Provider::native(&repository)?;
        run(&input.root, &provider, number)
    });
    match recovered {
        Ok(message) => super::report(&input, &message),
        Err(error) => {
            eprintln!("plumb follow --recover: {error}");
            1
        }
    }
}

pub(super) fn run(root: &Path, provider: &Provider<'_>, number: u64) -> Result<String, String> {
    let seat = Seat::open(provider.repository)?;
    settle(root, provider, &seat, number)
}

pub(super) fn settle(
    root: &Path,
    provider: &Provider<'_>,
    seat: &Seat,
    number: u64,
) -> Result<String, String> {
    let state = seat.read(provider.repository)?;
    let archive = seat.worktree()?.join(format!("recovered-{number}.json"));
    let previous = retained(&archive)?;
    if state.issue != number {
        if let Some(previous) = previous {
            let original: State =
                serde_json::from_str(&previous.state).map_err(|error| error.to_string())?;
            if original.issue == number && original.repository == provider.repository {
                provider.writer(number)?;
                return Ok(format!(
                    "Auto #{number} recovery already recorded at {}",
                    archive.display()
                ));
            }
        }
        return Err(format!(
            "Auto #{number} is outside the active recovery state"
        ));
    }

    let recovery = Recovery {
        root,
        provider,
        state: &state,
    };
    let evidence = recovery.observe()?;
    let local = recovery.local(seat, &evidence)?;
    let original = std::fs::read_to_string(&seat.path).map_err(|error| error.to_string())?;
    let receipt = Receipt {
        schema: "plumb.auto-recovery/v1".into(),
        state: original,
        local,
        head: evidence.head.clone(),
        merge: evidence.merge.clone(),
        guard: evidence.guard.clone(),
        candidate: evidence.candidate.clone(),
        history: evidence.history.clone(),
    };
    if let Some(previous) = previous {
        if previous != receipt {
            return Err("retained Auto recovery receipt disagrees".into());
        }
    } else {
        let mut file =
            tempfile::NamedTempFile::new_in(seat.worktree()?).map_err(|error| error.to_string())?;
        serde_json::to_writer(&mut file, &receipt).map_err(|error| error.to_string())?;
        file.flush()
            .and_then(|()| file.as_file().sync_all())
            .map_err(|error| error.to_string())?;
        file.persist_noclobber(&archive)
            .map_err(|error| error.to_string())?;
    }
    if recovery.observe()? != evidence
        || recovery.local(seat, &evidence)? != receipt.local
        || std::fs::read_to_string(&seat.path).map_err(|error| error.to_string())? != receipt.state
    {
        return Err(
            "Auto recovery evidence moved before settlement; inspect retained receipt".into(),
        );
    }
    seat.write(&State {
        schema: state.schema,
        repository: state.repository,
        ..State::default()
    })?;
    Ok(format!(
        "Auto #{number} recovered at {}; old worktree and branch retained. Rerun plumb follow.",
        archive.display()
    ))
}

pub(super) fn checkout(root: &Path, seat: &Seat, state: &State) -> Result<PathBuf, String> {
    let parent = seat.worktree()?;
    let legacy = parent.join("worktree");
    if !std::fs::symlink_metadata(&legacy).is_ok_and(|metadata| metadata.is_dir()) {
        return Ok(legacy);
    }
    let branch = work::git(&legacy, &["branch", "--show-current"])?;
    let number = branch
        .strip_prefix("auto/")
        .and_then(|number| number.parse::<u64>().ok());
    let Some(number) = number.filter(|number| *number != state.issue) else {
        return Ok(legacy);
    };
    let Some(receipt) = retained(&parent.join(format!("recovered-{number}.json")))? else {
        return Ok(legacy);
    };
    let original: State =
        serde_json::from_str(&receipt.state).map_err(|error| error.to_string())?;
    if (&original.worktree, &original.repository, original.issue)
        != (&legacy, &state.repository, number)
    {
        return Err("retained Auto receipt disagrees with the legacy worktree".into());
    }
    work::Work {
        source: root,
        state: &original,
    }
    .inspect()?;
    Ok(parent.join(format!("worktree-{}", state.issue)))
}

fn retained(path: &Path) -> Result<Option<Receipt>, String> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => {
            let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
            let receipt: Receipt =
                serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
            if receipt.schema != "plumb.auto-recovery/v1" {
                return Err("retained Auto recovery schema disagrees".into());
            }
            Ok(Some(receipt))
        }
        Ok(_) => Err("Auto recovery receipt must be a regular file".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}
