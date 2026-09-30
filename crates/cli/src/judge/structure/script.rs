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
    for (tool, version, held) in &evidence.engines {
        let seen = held.as_deref().unwrap_or("missing");
        found.push(wrong(
            &rule::ENGINE_DOMAIN_EXACT,
            format!("package.json engines.{tool} is {seen}; declare exactly {version:?}"),
        ));
    }
    if let Some(held) = &evidence.manager {
        found.push(wrong(
            &rule::PACKAGE_MANAGER_ABSENT,
            format!(
                "package.json declares packageManager {held}; remove it and declare exact engines"
            ),
        ));
    }
    found
}
