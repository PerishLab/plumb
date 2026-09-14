use super::{Catalog, Configuration, FACTORY, Legacy, inline, profiled};
use std::collections::BTreeSet;

pub(super) fn names<C: Configuration>(seat: &C) -> Result<Vec<String>, String> {
    if seat.mark().is_none() {
        return Err("ship authority requires a verified Depot configuration".into());
    }
    let raw = seat.read("rules/products.toml", FACTORY)?;
    let table: toml::Table = raw
        .parse()
        .map_err(|error| format!("invalid catalog: {error}"))?;
    let schema = table.get("schema").and_then(toml::Value::as_str);
    let identities = match schema {
        Some("plumb.products/v1") => {
            let catalog: Legacy =
                toml::from_str(&raw).map_err(|error| format!("invalid catalog: {error}"))?;
            catalog.validate()?;
            catalog
                .product
                .into_iter()
                .map(|product| product.identity)
                .collect::<Vec<_>>()
        }
        Some("plumb.products/v2") => {
            let catalog: Catalog =
                toml::from_str(&raw).map_err(|error| format!("invalid catalog: {error}"))?;
            catalog.validate()?;
            catalog
                .product
                .into_iter()
                .map(|product| product.identity)
                .collect::<Vec<_>>()
        }
        _ => return Err("ship authority requires a supported product catalog".into()),
    };
    let mut names = BTreeSet::new();
    for identity in identities {
        let product = match schema {
            Some("plumb.products/v1") => inline(&raw, &identity, seat)?,
            _ => profiled(&raw, &identity, seat)?,
        };
        if !names.insert(product.product) {
            return Err("ship authority catalog repeats a product name".into());
        }
    }
    if names.is_empty() {
        return Err("ship authority catalog names no products".into());
    }
    Ok(names.into_iter().collect())
}
