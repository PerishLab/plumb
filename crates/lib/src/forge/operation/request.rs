use super::super::Digest;
use super::super::evidence::{Context, Reason};
use super::{Assessment, Authorization, Intent, State, Writer};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Request(Parts);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Parts {
    context: Context,
    intent: Intent,
    expected: Option<State>,
}

impl Request {
    pub fn new(context: Context, intent: Intent, expected: Option<State>) -> Result<Self, String> {
        checked(Parts {
            context,
            intent,
            expected,
        })
    }

    pub fn context(&self) -> &Context {
        &self.0.context
    }

    pub fn intent(&self) -> &Intent {
        &self.0.intent
    }

    pub fn expected(&self) -> Option<&State> {
        self.0.expected.as_ref()
    }

    pub fn digest(&self) -> Result<Digest, String> {
        let bytes = serde_json::to_vec(self).map_err(|error| error.to_string())?;
        Digest::read(&crate::depot::sha(&bytes))
    }

    pub fn admit(&self, assessment: &Assessment) -> Result<(), Reason> {
        if &self.digest().map_err(|_| Reason::Unverified)? != assessment.request() {
            return Err(Reason::Conflict);
        }
        if !assessment.capabilities().supports(self.intent().action()) {
            return Err(Reason::Unsupported);
        }
        match assessment.authorization() {
            Authorization::Granted { .. } => {}
            Authorization::Denied { .. } => return Err(Reason::Denied),
            Authorization::Unknown {} => return Err(Reason::Unverified),
        }
        if !self.intent().mutates() {
            return Ok(());
        }
        match assessment.writer() {
            Writer::Idle => {}
            Writer::Active => return Err(Reason::Conflict),
            Writer::Unknown => return Err(Reason::Unavailable),
        }
        if matches!(assessment.observed(), State::Unknown {}) {
            return Err(Reason::Unverified);
        }
        if self.expected() != Some(assessment.observed()) {
            return Err(Reason::Conflict);
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for Request {
    fn deserialize<D: Deserializer<'de>>(reader: D) -> Result<Self, D::Error> {
        checked(Parts::deserialize(reader)?).map_err(serde::de::Error::custom)
    }
}

fn checked(parts: Parts) -> Result<Request, String> {
    if let Some(expected) = &parts.expected {
        expected.validate()?;
    }
    match (&parts.intent, &parts.expected) {
        (Intent::Inspect {} | Intent::Build { .. }, None) => {}
        (Intent::Publish { .. }, Some(State::Absent {})) => {}
        (
            Intent::Install { .. } | Intent::Deploy { .. },
            Some(State::Absent {} | State::Present { .. }),
        ) => {}
        (Intent::Uninstall {} | Intent::Dispose {}, Some(State::Present { .. })) => {}
        _ => return Err("lane action needs its explicit atomic precondition".into()),
    }
    Ok(Request(parts))
}
