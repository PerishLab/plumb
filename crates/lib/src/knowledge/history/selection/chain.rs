use super::{Choice, Family, Fault, Inspector, Read, Reference, Selection, Selector, Wire};
use crate::semantics::version::Markers;
use std::collections::BTreeSet;

impl<R: Read> Inspector<R> {
    pub(super) fn choices(
        &self,
        selection: &Selection,
        family: &Family,
    ) -> Result<Vec<Choice>, Fault> {
        let mut result = Vec::new();
        let mut seen = BTreeSet::new();
        let mut next = selection.previous.clone();
        let mut child = selection.clone();
        let mut bytes = 0_u64;
        while let Some(reference) = next {
            if result.len() >= 128 || bytes.saturating_add(reference.size) > 16 * 1024 * 1024 {
                return Err(Fault::unknown(
                    "selection predecessor observation bounds exhausted",
                ));
            }
            Wire::require(
                seen.insert(reference.sha256.clone()),
                "selection repeats a predecessor",
            )?;
            let choice = self.choice(&reference, &selection.product)?;
            let held = choice.record();
            Wire::require(
                held.sequence.checked_add(1) == Some(child.sequence)
                    && held.root == selection.root
                    && held.product == selection.product,
                "selection sequence or original root changed",
            )?;
            let old = Self::position(&held.family, family)?;
            let current = Self::position(&child.family, family)?;
            Wire::require(
                old < current
                    && Markers::stable(&held.marker).map_err(Fault::refused)?
                        < Markers::stable(&child.marker).map_err(Fault::refused)?,
                "selection replaced its branch or did not advance stable",
            )?;
            let origin = self.record(&held.family, &selection.product)?;
            Wire::require(
                origin.record.marker() == held.marker,
                "selection predecessor names another formal marker",
            )?;
            let commit = origin.record.release()?.snapshot.source.commit;
            let history = self.history(&held.history, &selection.product)?;
            Wire::require(
                history.root().origin.reference == selection.root
                    && history.checkpoints()[0]
                        .markers
                        .iter()
                        .any(|tag| tag.marker == held.marker && tag.commit == commit),
                "selection predecessor history omitted its stable or original root",
            )?;
            bytes += reference.size;
            next = held.previous.clone();
            child = held.clone();
            result.push(choice);
        }
        Wire::require(
            child.sequence == 1,
            "selection predecessor history is incomplete",
        )?;
        Ok(result)
    }

    pub(super) fn inception(
        &self,
        selection: &Selection,
        choices: &[Choice],
    ) -> Result<Option<Selector>, Fault> {
        let first = Selector::first(&self.0, &selection.product)?;
        let expected = match choices.last() {
            Some(choice) => choice.reference().clone(),
            None => selection.reference()?,
        };
        if let Some(first) = &first {
            Wire::require(
                first.reference == expected,
                "selection replaced its original inception",
            )?;
        } else if selection.previous.is_some() {
            return Err(Fault::unknown("selection inception is undiscovered"));
        }
        Ok(first)
    }

    fn position(reference: &Reference, family: &Family) -> Result<usize, Fault> {
        family
            .records
            .iter()
            .position(|held| held == reference)
            .ok_or_else(|| Fault::refused("selection left its verified family branch"))
    }
}
