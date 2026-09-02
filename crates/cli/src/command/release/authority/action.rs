use plumb::forgejo::Client;
use std::{thread, time::Duration};

use super::cloudflare::Grant;
use super::{Action, ITEM, context::Context, escrow::Escrow};

impl Context {
    pub fn apply(&self, action: Action) -> Result<(), String> {
        match action {
            Action::Bucket => self.bucket(),
            Action::Domain => self.domain(),
            Action::Capability => self.capability(),
            Action::Recovery => self.recovery(),
            Action::Repository => self.repository(),
        }
    }

    fn bucket(&self) -> Result<(), String> {
        self.session(|bucket| {
            if bucket.live()? {
                return Err("release bucket appeared after planning".into());
            }
            bucket.create()?;
            if bucket.live()? {
                Ok(())
            } else {
                Err("release bucket creation did not verify".into())
            }
        })
    }

    fn domain(&self) -> Result<(), String> {
        self.session(|bucket| {
            if !bucket.live()? {
                return Err("release bucket disappeared after planning".into());
            }
            bucket.bind(&self.model.domain, &self.model.zone)?;
            for turn in 0..8 {
                if bucket
                    .find(&self.model.domain)?
                    .is_some_and(|held| held.ready())
                {
                    return Ok(());
                }
                if turn < 7 {
                    thread::sleep(Duration::from_secs(5));
                }
            }
            Err("release domain did not become active within 40 seconds".into())
        })
    }

    fn capability(&self) -> Result<(), String> {
        if self
            .factory
            .held()?
            .iter()
            .any(|token| token.name == self.model.writer())
            || self.model.escrow.exists()
        {
            return Err("release writer appeared after planning".into());
        }
        let permission = self.factory.permission(ITEM.0, ITEM.1)?;
        let minted = self.factory.create(&Grant {
            name: self.model.writer(),
            permission,
            resource: format!(
                "com.cloudflare.edge.r2.bucket.{}_default_{}",
                self.factory.id(),
                self.model.bucket
            ),
            expires: String::new(),
        })?;
        let held = Escrow::minted(
            minted.id.clone(),
            minted.value(),
            self.model.bucket.clone(),
            self.factory.id(),
        );
        let seat = super::escrow::Seat::new(&self.model.escrow);
        if let Err(error) = held.verify().and_then(|()| seat.write(&held)) {
            return match self.factory.revoke(&minted.id) {
                Ok(()) => Err(error),
                Err(revoke) => Err(format!("{error}; writer rollback also failed: {revoke}")),
            };
        }
        Ok(())
    }

    fn recovery(&self) -> Result<(), String> {
        let writers = self
            .factory
            .held()?
            .into_iter()
            .filter(|token| token.name == self.model.writer())
            .map(|token| token.id)
            .collect::<Vec<_>>();
        let seat = super::escrow::Seat::new(&self.model.escrow);
        if let Some(held) = seat.load()?
            && writers.iter().any(|id| id == &held.access)
        {
            held.exact(&self.model.bucket, self.factory.id())?;
            held.verify()?;
            self.store(&held)?;
            return self.prune(&writers, &held.access);
        }

        let permission = self.factory.permission(ITEM.0, ITEM.1)?;
        let minted = self.factory.create(&Grant {
            name: self.model.writer(),
            permission,
            resource: format!(
                "com.cloudflare.edge.r2.bucket.{}_default_{}",
                self.factory.id(),
                self.model.bucket
            ),
            expires: String::new(),
        })?;
        let held = Escrow::minted(
            minted.id.clone(),
            minted.value(),
            self.model.bucket.clone(),
            self.factory.id(),
        );
        if let Err(error) = held.verify().and_then(|()| seat.replace(&held)) {
            return match self.factory.revoke(&minted.id) {
                Ok(()) => Err(error),
                Err(revoke) => Err(format!("{error}; writer rollback also failed: {revoke}")),
            };
        }
        self.store(&held)?;
        self.prune(&writers, &held.access)
    }

    fn prune(&self, writers: &[String], retained: &str) -> Result<(), String> {
        for id in writers.iter().filter(|id| id.as_str() != retained) {
            self.factory.revoke(id)?;
        }
        Ok(())
    }

    fn repository(&self) -> Result<(), String> {
        let held = super::escrow::Seat::new(&self.model.escrow)
            .load()?
            .ok_or_else(|| "release escrow disappeared after planning".to_string())?;
        self.store(&held)
    }

    fn store(&self, held: &Escrow) -> Result<(), String> {
        held.exact(&self.model.bucket, self.factory.id())?;
        let client = Client::new(self.model.remote.clone())?;
        let inventory = self.model.inventory();
        let tail = match self.model.profile {
            "release" => super::escrow::fingerprint(&held.endpoint),
            _ => inventory,
        };
        let values = [
            held.access.as_str(),
            held.secret.as_str(),
            held.bucket.as_str(),
            held.endpoint.as_str(),
            tail.as_str(),
        ];
        for (name, value) in self.model.secrets().iter().zip(values) {
            match self.model.organization() {
                Some(owner) => client.store(owner, name, value)?,
                None => client.set(name, value)?,
            };
        }
        let present = match self.model.organization() {
            Some(owner) => client.held(owner)?,
            None => client.secrets()?,
        };
        if self
            .model
            .secrets()
            .iter()
            .all(|name| present.contains(*name))
        {
            Ok(())
        } else {
            Err("Forgejo publish-secret upsert did not verify".into())
        }
    }
}
