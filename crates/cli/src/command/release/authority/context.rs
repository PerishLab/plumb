use plumb::{config::Cascade as _, forgejo::Client, rig::Mint};

use super::cloudflare::{Bucket, Factory, Grant};
use super::{ADMIN, Observation, escrow::Escrow, model::Model};

pub(super) struct Context {
    pub model: Model,
    pub(super) factory: Factory,
}

impl Context {
    pub fn open(model: Model) -> Result<Self, String> {
        let held = Mint::default().merge(Mint::env("PLUMB_AUTHORITY").map_err(|e| e.to_string())?);
        if held.account.trim().is_empty() || held.token.trim().is_empty() {
            return Err("missing PLUMB_AUTHORITY_ACCOUNT or PLUMB_AUTHORITY_TOKEN".into());
        }
        if held.token.contains(['\r', '\n']) {
            return Err("PLUMB_AUTHORITY_TOKEN contains a line break".into());
        }
        Ok(Self {
            model,
            factory: Factory::new(
                held.account,
                held.api.trim_end_matches('/').to_string(),
                held.token,
            ),
        })
    }

    pub fn observe(&self) -> Result<Observation, String> {
        self.factory.verify()?;
        let held = self.factory.held()?;
        let writers = held
            .iter()
            .filter(|token| token.name == self.model.writer())
            .collect::<Vec<_>>();
        let capability = match writers.as_slice() {
            [] => None,
            [token] => Some(token.id.clone()),
            _ => return Err(format!("duplicate writer {}", self.model.writer())),
        };
        let seat = super::escrow::Seat::new(&self.model.escrow);
        let escrow = seat.load()?;
        self.writer(capability.as_deref(), escrow.as_ref())?;
        let (bucket, domain) = self.session(|bucket| {
            let live = bucket.live()?;
            let domain = if live {
                bucket.find(&self.model.domain)?
            } else {
                None
            };
            Ok((live, domain))
        })?;
        if let Some(domain) = &domain
            && domain.zone != self.model.zone
        {
            return Err(format!("release domain belongs to zone {}", domain.zone));
        }
        let secrets = Client::new(self.model.remote.clone())?.secrets()?;
        Ok(Observation {
            bucket,
            domain,
            capability,
            escrow: escrow.as_ref().map(Escrow::view),
            secrets,
        })
    }

    fn writer(&self, id: Option<&str>, held: Option<&Escrow>) -> Result<(), String> {
        match (id, held) {
            (None, None) => Ok(()),
            (Some(id), Some(held)) if id == held.access => {
                held.exact(&self.model.bucket, self.factory.id())?;
                held.verify()
            }
            (Some(_), None) => Err(format!(
                "writer exists but {} cannot recover its one-time secret",
                self.model.escrow.display()
            )),
            (None, Some(_)) => Err("release escrow names a missing writer".into()),
            (Some(_), Some(_)) => Err("writer and release escrow disagree".into()),
        }
    }

    pub(super) fn session<T>(
        &self,
        operation: impl FnOnce(&Bucket<'_>) -> Result<T, String>,
    ) -> Result<T, String> {
        for stale in self
            .factory
            .held()?
            .iter()
            .filter(|held| held.name == self.model.temporary())
        {
            self.factory.revoke(&stale.id)?;
        }
        let permission = self.factory.permission(ADMIN.0, ADMIN.1)?;
        let minted = self.factory.create(&Grant {
            name: self.model.temporary(),
            permission,
            resource: format!("com.cloudflare.api.account.{}", self.factory.id()),
            expires: crate::command::clock::ahead(15)?,
        })?;
        let result = operation(&Bucket::new(&self.factory, &minted, &self.model.bucket));
        let revoked = self.factory.revoke(&minted.id);
        match (result, revoked) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), Ok(())) => Err(error),
            (Ok(_), Err(error)) => Err(error),
            (Err(error), Err(revoke)) => Err(format!(
                "{error}; temporary token cleanup also failed: {revoke}"
            )),
        }
    }
}
