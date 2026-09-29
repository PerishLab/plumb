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
    found
}
