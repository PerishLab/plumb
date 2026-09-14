use plumb::forgejo::Client;

use super::cloudflare::{Bucket, Factory, Grant, Resource};
use super::{ADMIN, Observation, escrow::Escrow, model::Model};

pub(super) struct Context {
    pub model: Model,
    pub(super) factory: Factory,
}

impl Context {
    pub fn open(model: Model) -> Result<Self, String> {
        Ok(Self {
            model,
            factory: Factory::resolve()?,
        })
    }

    pub fn observe(&self) -> Result<Observation, String> {
        self.factory.verify()?;
        if self.model.profile == "depot" {
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
                return Err(format!("depot domain belongs to zone {}", domain.zone));
            }
            return Ok(Observation {
                bucket,
                domain,
                capability: None,
                recovery: false,
                escrow: None,
                secrets: Default::default(),
                policy: None,
                note: None,
            });
        }
        let held = self.factory.held()?;
        let writers = held
            .iter()
            .filter(|token| token.name == self.model.writer())
            .map(|token| token.id.clone())
            .collect::<Vec<_>>();
        let capability = match writers.as_slice() {
            [] => None,
            [token] => Some(token.clone()),
            _ if self.model.recovery => None,
            _ => return Err(format!("duplicate writer {}", self.model.writer())),
        };
        let (escrow, recovery, note) = self.escrow(&writers)?;
        let policy = if self.model.profile == "ship" {
            capability
                .as_ref()
                .map(|id| self.factory.policy(id, &self.model.buckets))
                .transpose()?
        } else {
            None
        };
        let (bucket, domain) = if self.model.profile == "ship" {
            (true, None)
        } else {
            self.session(|bucket| {
                let live = bucket.live()?;
                let domain = if live {
                    bucket.find(&self.model.domain)?
                } else {
                    None
                };
                Ok((live, domain))
            })?
        };
        if let Some(domain) = &domain
            && domain.zone != self.model.zone
        {
            return Err(format!(
                "{} domain belongs to zone {}",
                self.model.profile, domain.zone
            ));
        }
        let forge = Client::new(self.model.remote.clone())?;
        let secrets = match self.model.organization() {
            Some(owner) => forge.held(owner)?,
            None => forge.secrets()?,
        };
        Ok(Observation {
            bucket,
            domain,
            capability,
            recovery,
            escrow: escrow.as_ref().map(Escrow::view),
            secrets,
            policy,
            note,
        })
    }

    fn escrow(&self, writers: &[String]) -> Result<(Option<Escrow>, bool, Option<String>), String> {
        if self.model.profile == "ship" && !self.model.recovery && writers.len() == 1 {
            return match self.binding(&writers[0]) {
                Ok(held) => Ok((Some(held), false, None)),
                Err(note) => Ok((None, false, Some(note))),
            };
        }
        let held = super::escrow::Seat::new(&self.model.escrow).load()?;
        let recovery = self.writer(writers, held.as_ref())?;
        Ok((held, recovery, None))
    }

    fn binding(&self, writer: &str) -> Result<Escrow, String> {
        let held = super::escrow::Seat::new(&self.model.escrow)
            .load()?
            .ok_or("local escrow is absent; existing remote credentials remain opaque")?;
        if held.access != writer {
            return Err("local escrow does not name the existing writer".into());
        }
        held.exact(&self.model.bucket, self.factory.id())?;
        held.verify()?;
        Ok(held)
    }

    fn writer(&self, writers: &[String], held: Option<&Escrow>) -> Result<bool, String> {
        if self.model.recovery {
            if let Some(held) = held
                && writers.iter().any(|id| id == &held.access)
            {
                held.exact(&self.model.bucket, self.factory.id())?;
                held.verify()?;
                return Ok(writers.len() != 1);
            }
            return Ok(!writers.is_empty() || held.is_some());
        }
        match (writers, held) {
            ([], None) => Ok(false),
            ([id], Some(held)) if id == &held.access => {
                held.exact(&self.model.bucket, self.factory.id())?;
                held.verify()?;
                Ok(false)
            }
            ([_], None) => Err(format!(
                "writer exists but {} cannot recover its one-time secret",
                self.model.escrow.display()
            )),
            ([], Some(_)) => Err("release escrow names a missing writer".into()),
            ([_], Some(_)) => Err("writer and release escrow disagree".into()),
            _ => Err(format!("duplicate writer {}", self.model.writer())),
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
            resource: Resource::Exact(format!("com.cloudflare.api.account.{}", self.factory.id())),
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
