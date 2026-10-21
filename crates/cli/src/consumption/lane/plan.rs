use super::{Planning, Reviewed, Selection, Status, hub, status};
use plumb::lane::{Digest, Source, declaration::Declaration, dispatch::Dispatch};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::Path;

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Plan {
    pub schema: String,
    pub dispatch: Dispatch,
    pub declaration: Declaration,
    pub source: Source,
}

impl Plan {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != "plumb.lane.plan/v1"
            || self.source.repository() != self.dispatch.target().repository()
        {
            return Err("invalid static lane plan identity".into());
        }
        if self.dispatch.operation() == plumb::lane::dispatch::Operation::Inspect {
            return Err(
                "remote inspect dispatch is unsupported; use immutable lane inspect".into(),
            );
        }
        self.dispatch.qualify(&self.declaration)?;
        if let Some(source) = self.dispatch.source()
            && (self.source.commit() != source.commit || self.source.tree() != source.tree)
        {
            return Err("lane apply source differs from the exact qualified source".into());
        }
        Ok(())
    }

    pub fn digest(&self) -> Result<Digest, String> {
        let bytes = serde_json::to_vec(self).map_err(|_| "cannot encode lane plan")?;
        Digest::read(&plumb::depot::sha(&bytes))
    }

    pub fn observe(&self, root: &str) -> Result<(), String> {
        let observed = status::read(Status {
            target: plumb::cli::Root { root: root.into() },
            selection: Selection {
                app: self.dispatch.target().app().into(),
                lane: self.dispatch.target().lane().value().into(),
            },
            require: Some(self.dispatch.capability()),
            json: true,
        })?;
        let source: Source = serde_json::from_value(observed["source"].clone())
            .map_err(|_| "invalid lane source")?;
        let declaration: Declaration = serde_json::from_value(observed["declaration"].clone())
            .map_err(|_| "invalid lane declaration")?;
        if source != self.source || declaration != self.declaration {
            return Err("lane plan source or declaration moved; prepare and review again".into());
        }
        Ok(())
    }
}

pub(super) fn prepare(options: Planning) -> Result<serde_json::Value, String> {
    let dispatch = Dispatch::read(&bounded(&options.input)?)?;
    let observed = status::read(Status {
        target: options.target,
        selection: options.selection,
        require: Some(dispatch.capability()),
        json: true,
    })?;
    let plan = Plan {
        schema: "plumb.lane.plan/v1".into(),
        dispatch,
        declaration: serde_json::from_value(observed["declaration"].clone())
            .map_err(|_| "invalid lane declaration")?,
        source: serde_json::from_value(observed["source"].clone())
            .map_err(|_| "invalid lane source")?,
    };
    plan.validate()?;
    hub::observe(&plan)?;
    Ok(
        serde_json::json!({"plan": plan, "digest": plan.digest()?, "current":{"outcome":"unknown","reason":"unverified"}}),
    )
}

pub(super) fn reviewed(options: &Reviewed) -> Result<Plan, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Report {
        plan: Plan,
        digest: Digest,
        current: plumb::lane::evidence::Observation,
    }
    let report: Report = serde_json::from_slice(&bounded(&options.plan)?)
        .map_err(|_| "invalid reviewed lane plan")?;
    report.plan.validate()?;
    let expected = Digest::read(&options.digest)?;
    if report.digest != expected || report.plan.digest()? != expected {
        return Err("lane plan differs from the explicitly reviewed digest".into());
    }
    if report.current
        != (plumb::lane::evidence::Observation::Unknown {
            reason: plumb::lane::evidence::Reason::Unverified,
        })
    {
        return Err("a prepared lane plan cannot attest remote current state".into());
    }
    Ok(report.plan)
}

pub(super) fn bounded(path: &Path) -> Result<Vec<u8>, String> {
    let file = std::fs::File::open(path).map_err(|_| "cannot read bounded lane document")?;
    let mut bytes = Vec::new();
    file.take(65537)
        .read_to_end(&mut bytes)
        .map_err(|_| "cannot read bounded lane document")?;
    if bytes.len() > 65536 {
        return Err("lane document exceeds its budget".into());
    }
    Ok(bytes)
}
