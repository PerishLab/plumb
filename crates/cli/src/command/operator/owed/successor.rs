use super::{
    Seat, below,
    decision::{Decision, Result, require, stable},
    native::Native,
    origin,
};
use crate::catalog::rules::release as law;
use plumb::windtun::qualification::{Read, Remote};
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
pub(in crate::command::operator) enum Stage<'a> {
    Open(&'a str),
    Stamp(&'a str, &'a str),
    Dispatch(&'a str),
}

impl Stage<'_> {
    fn head(&self) -> &str {
        match self {
            Self::Open(head) | Self::Stamp(head, _) | Self::Dispatch(head) => head,
        }
    }
}

pub(super) struct Request<'a> {
    pub blob: &'a str,
    pub marker: &'a str,
    pub stage: Stage<'a>,
}

impl Seat<'_> {
    pub(in crate::command::operator) fn successor(
        &self,
        blob: &str,
        marker: &str,
        stage: Stage<'_>,
    ) -> Result<String> {
        self.inspect(
            &Native(&Remote::default()),
            Request {
                blob,
                marker,
                stage,
            },
            |old| {
                Ok(self
                    .statuses(old)?
                    .into_iter()
                    .filter(|(_, settled)| !settled)
                    .map(|(o, _)| o.rule.0)
                    .collect())
            },
        )
    }

    pub(super) fn inspect<R: Read>(
        &self,
        native: &Native<'_, R>,
        request: Request<'_>,
        debts: impl FnOnce(&plumb::land::rejoin::Stable) -> Result<Vec<&'static str>>,
    ) -> Result<String> {
        let Request {
            blob,
            marker,
            stage,
        } = request;
        let (repo, push) = origin::repository(self)?;
        let id = native.product(&repo)?;
        let base = marker.strip_suffix("-rc.1").unwrap_or(marker);
        stable(base)?;
        let (authority, decision) = native.decision(id, base, blob)?;
        decision.check(&repo, id)?;
        require(
            decision.successor.marker == base && stage.head() == decision.successor.commit,
            "requested marker or source differs from authorized successor",
        )?;
        let old =
            below(self.listing, marker).ok_or("specified successor: original stable missing")?;
        require(
            old.marker == decision.predecessor.marker && old.commit == decision.predecessor.commit,
            "highest prior stable differs from the decision",
        )?;
        native.manifests(&decision, self)?;
        native.history(&decision)?;
        native.stage(&decision, marker, &stage)?;
        let debts = debts(&old)?;
        require(
            debts == [law::MARKER_DISTRIBUTED.0],
            "expected only the original incomplete-distribution obligation",
        )?;
        native.media(&decision, &self.spec.product)?;
        if marker == decision.successor.marker && !matches!(stage, Stage::Open(_)) {
            native.promotion(&decision)?;
        }
        native.unspent(&decision, marker, &stage)?;
        native.stage(&decision, marker, &stage)?;
        let tags = native.refs(id, "tags/v")?;
        let local = self
            .listing
            .lines()
            .filter_map(|s| s.split_once('\t'))
            .filter(|(_, key)| key.starts_with("refs/tags/v") && !key.ends_with("^{}"))
            .map(|(v, k)| (k.to_string(), v.to_string()))
            .collect::<BTreeMap<_, _>>();
        require(
            tags == local,
            "native marker listing differs from the selected remote",
        )?;
        require(
            native.main()? == authority,
            "organization main changed during successor qualification",
        )?;
        require(
            origin::repository(self)? == (repo, push.clone()),
            "Git remote changed during successor qualification",
        )?;
        eprintln!(
            "specified successor {} at {} applies to {} at {} (organization main {}, blob {})",
            marker,
            stage.head(),
            old.marker,
            old.commit,
            authority,
            blob
        );
        Ok(push)
    }
}

impl<R: Read> Native<'_, R> {
    fn stage(&self, d: &Decision, marker: &str, stage: &Stage<'_>) -> Result<()> {
        let id = d.identity;
        let next = &d.successor;
        let lines = self.refs(id, "heads/release/")?;
        require(
            !lines.contains_key(&format!("refs/heads/release/{}", d.predecessor.marker)),
            "original release line remains open",
        )?;
        let line = lines.get(&format!("refs/heads/release/{}", next.marker));
        if let Stage::Open(_) = stage {
            require(
                marker == next.marker && line.is_none(),
                "unexpected open successor line",
            )?;
            return self.branch(id, "main", &next.commit);
        }
        require(line == Some(&next.commit), "native successor line differs")?;
        let expected = if marker == next.rc {
            marker.to_string()
        } else {
            format!("{marker}\n\npromotes {}", next.rc)
        };
        match stage {
            Stage::Stamp(_, message) => require(
                *message == expected,
                "planned stamp or RC promotion differs",
            ),
            Stage::Dispatch(_) => origin::annotated(&self.immutable(id, marker)?, marker, d),
            Stage::Open(_) => unreachable!(),
        }
    }

    fn unspent(&self, d: &Decision, marker: &str, stage: &Stage<'_>) -> Result<()> {
        let next = &d.successor;
        self.existing(
            d,
            &next.marker,
            matches!(stage, Stage::Dispatch(_)) && marker == next.marker,
        )?;
        if marker == next.rc || matches!(stage, Stage::Open(_)) {
            self.existing(
                d,
                &next.rc,
                matches!(stage, Stage::Dispatch(_)) && marker == next.rc,
            )?;
        }
        Ok(())
    }

    fn existing(&self, d: &Decision, marker: &str, retry: bool) -> Result<()> {
        let channel = if marker == d.successor.rc {
            "rc"
        } else {
            "stable"
        };
        let url = format!(
            "{}/v1/releases/{channel}/{marker}/distribution.json",
            d.authority
        );
        let Some(raw) = self
            .0
            .optional(&url, 1024 * 1024)
            .map_err(|e| format!("specified successor: {e}"))?
        else {
            return Ok(());
        };
        let v: serde_json::Value = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
        require(
            v["schema"] == 1 && v["repository"] == d.repository && v["marker"] == marker,
            "successor receipt identity differs",
        )?;
        require(
            v["commit"] == d.successor.commit && v["tree"] == d.successor.tree,
            "existing successor receipt source differs",
        )?;
        require(
            retry && v["state"] == "incomplete",
            "successor already completed or was previously published at this stage",
        )
    }

    fn promotion(&self, d: &Decision) -> Result<()> {
        let next = &d.successor;
        origin::annotated(&self.immutable(d.identity, &next.rc)?, &next.rc, d)?;
        let raw = self.bytes(&format!(
            "{}/v1/releases/rc/{}/distribution.json",
            d.authority, next.rc
        ))?;
        let v: serde_json::Value = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
        require(
            v["schema"] == 1 && v["repository"] == d.repository && v["marker"] == next.rc,
            "RC distribution identity differs",
        )?;
        require(
            v["state"] == "complete" && v["commit"] == next.commit && v["tree"] == next.tree,
            "approved RC has no complete distribution at the exact source",
        )
    }

    fn media(&self, d: &Decision, product: &str) -> Result<()> {
        let auth = &d.authority;
        let old = &d.predecessor.marker;
        d.receipt(&self.bytes(&format!(
            "{auth}/v1/releases/stable/{old}/distribution.json"
        ))?)?;
        d.channel(
            &self.bytes(&format!("{auth}/v1/channels/stable.json"))?,
            product,
        )?;
        Ok(())
    }
}
