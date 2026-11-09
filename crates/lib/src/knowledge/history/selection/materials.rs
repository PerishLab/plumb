use super::{Choice, Family, Fault, History, Inspector, Read, Selection, Selector, Wire};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Nomination {
    schema: &'static str,
    selection: Selection,
    family: Family,
    history: History,
    predecessors: Vec<Choice>,
    inception: Option<Selector>,
    authentication: bool,
    publication: bool,
}

impl<R: Read> Inspector<R> {
    pub fn nominate(&self, selection: &Selection, product: &str) -> Result<Nomination, Fault> {
        let authority = self.current()?;
        let first = self.selection(selection, product)?;
        let inception = self.inception(selection, &first.2)?;
        let history = self.discover(product)?;
        Wire::require(
            history.reference() == &selection.history,
            "selection does not bind current retained history",
        )?;
        let inventory = self.markers(product)?;
        Self::coverage(&first.0, &history, &inventory)?;
        let repeated = self.selection(selection, product)?;
        Wire::require(
            Wire::canonical(&first)? == Wire::canonical(&repeated)?
                && self.markers(product)? == inventory
                && Wire::canonical(&self.discover(product)?)? == Wire::canonical(&history)?
                && self.inception(selection, &repeated.2)? == inception
                && self.current()? == authority,
            "selection materials, history, inventory or authority moved",
        )?;
        Ok(Nomination {
            schema: "plumb.release.selection.materials/v1",
            selection: selection.clone(),
            family: first.0,
            history,
            predecessors: first.2,
            inception,
            authentication: false,
            publication: false,
        })
    }

    pub(super) fn selection(
        &self,
        selection: &Selection,
        product: &str,
    ) -> Result<(Family, History, Vec<Choice>), Fault> {
        selection.encode()?;
        Wire::require(
            selection.product == product,
            "selection names another product",
        )?;
        let family = self.family(&selection.family, product)?;
        Wire::require(
            family.root() == &selection.root
                && family.stable().world().source.marker == selection.marker,
            "selection replaced its original root or formal stable",
        )?;
        let history = self.history(&selection.history, product)?;
        Wire::require(
            history.root().origin.reference == selection.root
                && family
                    .history
                    .as_ref()
                    .is_none_or(|reference| Self::extends(&history, reference)),
            "selection history replaced its root or discarded family history",
        )?;
        let source = &family.stable().world().source;
        Wire::require(
            history.checkpoints()[0]
                .markers
                .iter()
                .any(|tag| tag.marker == source.marker && tag.commit == source.commit),
            "selection history omitted or retargeted its stable",
        )?;
        let predecessors = self.choices(selection, &family)?;
        Ok((family, history, predecessors))
    }

    pub(super) fn extends(history: &History, reference: &super::Reference) -> bool {
        history.reference() == reference
            || history
                .checkpoints()
                .iter()
                .any(|checkpoint| checkpoint.previous.as_ref() == Some(reference))
    }
}
