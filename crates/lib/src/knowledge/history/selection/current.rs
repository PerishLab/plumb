use super::{Choice, Family, Fault, History, Inspector, Read, Reference, Selector, Wire};
use crate::semantics::admission::Inventory;
use crate::semantics::version::Markers;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Selected {
    schema: &'static str,
    selection: Choice,
    family: Family,
    history: History,
    inception: Selector,
    authentication: bool,
    publication: bool,
}

impl Selected {
    pub fn family(&self) -> &Family {
        &self.family
    }
    pub fn selection(&self) -> &Choice {
        &self.selection
    }
    pub fn history(&self) -> &History {
        &self.history
    }
}

impl<R: Read> Inspector<R> {
    pub fn selected(&self, root: &Reference, product: &str) -> Result<Selected, Fault> {
        root.validate(product)?;
        let authority = self.current()?;
        let selector = Selector::read(&self.0, product)?
            .ok_or_else(|| Fault::unknown("current stable family is undiscovered"))?;
        let choice = self.choice(&selector.reference, product)?;
        Wire::require(
            &choice.record().root == root,
            "current selection replaced expected original adoption",
        )?;
        let first = self.selection(choice.record(), product)?;
        let inception = self
            .inception(choice.record(), &first.2)?
            .ok_or_else(|| Fault::unknown("selection inception is undiscovered"))?;
        let history = self.discover(product)?;
        Wire::require(
            Self::extends(&history, &choice.record().history),
            "current history discarded selected history",
        )?;
        let inventory = self.markers(product)?;
        Self::coverage(&first.0, &history, &inventory)?;
        let repeated = self.selection(choice.record(), product)?;
        Wire::require(
            Wire::canonical(&first)? == Wire::canonical(&repeated)?
                && self.markers(product)? == inventory
                && Wire::canonical(&self.discover(product)?)? == Wire::canonical(&history)?
                && Wire::canonical(&self.choice(&selector.reference, product)?)?
                    == Wire::canonical(&choice)?
                && Selector::read(&self.0, product)? == Some(selector)
                && self.inception(choice.record(), &repeated.2)? == Some(inception.clone())
                && self.current()? == authority,
            "selected family, discovery, history, inventory or authority moved",
        )?;
        Ok(Selected {
            schema: "plumb.release.family.current/v1",
            selection: choice,
            family: first.0,
            history,
            inception,
            authentication: true,
            publication: false,
        })
    }

    pub(super) fn coverage(
        family: &Family,
        history: &History,
        inventory: &Inventory,
    ) -> Result<(), Fault> {
        let held = &history.checkpoints()[0].markers;
        if !inventory.markers.iter().all(|tag| held.contains(tag)) {
            return Err(Fault::unknown(
                "current history does not cover complete current tag inventory",
            ));
        }
        let mut latest = None;
        for tag in held {
            let version = Markers::parse(&tag.marker).map_err(Fault::refused)?;
            if version.pre.is_empty() && latest.as_ref().is_none_or(|prior| &version > prior) {
                latest = Some(version);
            }
        }
        let marker = latest
            .map(|version| format!("v{version}"))
            .ok_or_else(|| Fault::unknown("retained history has no formal stable"))?;
        let source = &family.stable().world().source;
        Wire::require(
            marker == source.marker,
            "selected family is not the latest retained stable",
        )?;
        if !inventory
            .markers
            .iter()
            .any(|tag| tag.marker == source.marker && tag.commit == source.commit)
        {
            return Err(Fault::unknown(
                "latest retained stable is absent or retargeted in current inventory",
            ));
        }
        Ok(())
    }
}
