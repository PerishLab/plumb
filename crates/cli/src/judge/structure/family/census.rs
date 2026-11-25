use super::{Widths, merge, store};
use std::collections::BTreeMap;

pub(super) enum Census {
    Entire(BTreeMap<String, usize>),
    Partial(BTreeMap<String, usize>),
    Blind(String),
}

pub(super) fn census(
    items: &[&str],
    mut widths: impl FnMut(&str) -> Result<BTreeMap<String, usize>, String>,
) -> Census {
    let mut found = BTreeMap::new();
    let mut errors = Vec::new();
    for name in items {
        match widths(name) {
            Ok(held) => merge(&mut found, held),
            Err(error) => errors.push(format!("{name}: {error}")),
        }
    }
    if errors.is_empty() {
        Census::Entire(found)
    } else if found.is_empty() {
        Census::Blind(errors.join("; "))
    } else {
        Census::Partial(found)
    }
}

pub(super) fn remembered(held: Census) -> Widths {
    if let Census::Entire(ref widths) = held {
        store(widths);
    }
    kept(held)
}

pub(super) fn kept(held: Census) -> Widths {
    match held {
        Census::Entire(held) | Census::Partial(held) => Widths::Known(held),
        Census::Blind(error) => Widths::Blind(error),
    }
}

#[cfg(test)]
pub(super) fn writable(held: &Census) -> bool {
    matches!(held, Census::Entire(_))
}
