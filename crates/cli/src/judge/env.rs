use super::finding::{Found, wrong};
use crate::catalog::rules::env as rule;
use crate::shape;

pub fn judge(held: &shape::Shape) -> Found {
    let mut found = Found::new();
    if held
        .edition
        .as_ref()
        .is_some_and(|edition| edition != "2024")
    {
        found.push(wrong(
            &rule::EDITION_2024,
            format!(
                "edition is {}, the skeleton holds 2024",
                held.edition.as_deref().unwrap_or("")
            ),
        ));
    }
    found
}
