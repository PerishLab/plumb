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
        if !referenced(image) {
            found.push(wrong(
                &rule::IMMUTABLE_CI_CONTAINER,
                format!(
                    "CI container {image} in {path} is neither a {FIRST}<name>:stable image nor pinned to @sha256:<64 lowercase hex>"
                ),
            ));
        }
    }
    for (tool, version, held) in &held.toolchain {
        let seen = match held {
            Ok(seen) if seen == version => continue,
            Ok(seen) => format!("reports {seen}"),
            Err(error) => format!("does not run ({error})"),
        };
        found.push(wrong(
            &rule::TOOLCHAIN_DOMAIN,
            format!(
                "{tool} {seen}, the domain runs {version}; see: plumb cookbook env.toolchain-domain"
            ),
        ));
    }
    found
}

const FIRST: &str = "ghcr.io/perishlab/";

fn referenced(image: &str) -> bool {
    match image.strip_prefix(FIRST) {
        Some(name) => name
            .strip_suffix(":stable")
            .is_some_and(|name| !name.is_empty() && !name.contains(['@', ':'])),
        None => immutable(image),
    }
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
