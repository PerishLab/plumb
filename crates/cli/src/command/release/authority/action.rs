use plumb::forgejo::Client;
use std::{thread, time::Duration};

use super::cloudflare::Grant;
use super::{Action, ITEM, SECRETS, context::Context, escrow::Escrow};

impl Context {
    pub fn apply(&self, action: Action) -> Result<(), String> {
        match action {
            Action::Bucket => self.bucket(),
            Action::Domain => self.domain(),
            Action::Capability => self.capability(),
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

    fn repository(&self) -> Result<(), String> {
        let held = super::escrow::Seat::new(&self.model.escrow)
            .load()?
            .ok_or_else(|| "release escrow disappeared after planning".to_string())?;
        held.exact(&self.model.bucket, self.factory.id())?;
        let client = Client::new(self.model.remote.clone())?;
        for (name, value) in [
            (SECRETS[0], held.access.as_str()),
            (SECRETS[1], held.secret.as_str()),
            (SECRETS[2], held.bucket.as_str()),
            (SECRETS[3], held.endpoint.as_str()),
        ] {
            client.set(name, value)?;
        }
        let present = client.secrets()?;
        if SECRETS.iter().all(|name| present.contains(*name)) {
            Ok(())
        } else {
            Err("Forgejo publish-secret upsert did not verify".into())
        }
    }
}
