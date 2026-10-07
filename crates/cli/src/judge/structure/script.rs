use super::super::finding::{Found, wrong};
use crate::catalog::rules::structure as rule;
use crate::shape::script::Evidence;

pub fn judge(evidence: &Evidence) -> Found {
    let mut found = Found::new();
    for (manifest, key, target) in &evidence.exports {
        found.push(wrong(
            &rule::SOURCE_EXPORT_EXPLICIT,
            format!("{manifest} exports {key} as {target}, a wildcard into src; name each subpath"),
        ));
    }
    for (test, package) in &evidence.tests {
        let seat = if package == "." {
            "tests/".to_string()
        } else {
            format!("{package}/tests/")
        };
        found.push(wrong(
            &rule::TEST_UNDER_TESTS,
            format!("test {test} sits outside {seat}"),
        ));
    }
    if let Some(held) = &evidence.manager {
        found.push(wrong(
            &rule::PACKAGE_MANAGER_ABSENT,
            format!(
                "package.json declares packageManager {held}; remove it, the domain versions are Plumb's (plumb metadata); see: plumb cookbook env.toolchain-domain"
            ),
        ));
    }
    if let Some(held) = &evidence.engines {
        found.push(wrong(
            &rule::ENGINES_ABSENT,
            format!(
                "package.json declares engines {held}; remove it, the domain versions are Plumb's (plumb metadata); see: plumb cookbook env.toolchain-domain"
            ),
        ));
    }
    for path in &evidence.pins {
        found.push(wrong(
            &rule::TOOLCHAIN_FILE_ABSENT,
            format!(
                "{path} pins a Rust toolchain; remove it, the domain Rust is Plumb's (plumb metadata rust.version); see: plumb cookbook env.toolchain-domain"
            ),
        ));
    }
    found
}
