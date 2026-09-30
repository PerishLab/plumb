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
    for (path, image) in &held.containers {
        if !immutable(image) {
            found.push(wrong(
                &rule::IMMUTABLE_CI_CONTAINER,
                format!(
                    "CI container {image} in {path} is not pinned to @sha256:<64 lowercase hex>"
                ),
            ));
        }
    }
    found
}

fn immutable(image: &str) -> bool {
    let Some((name, digest)) = image.rsplit_once("@sha256:") else {
        return false;
    };
    !name.is_empty()
        && digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
