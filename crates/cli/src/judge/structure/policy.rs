use crate::catalog::rules::structure as rule;
use crate::judge::finding::{Found, wrong};
use crate::shape::policy::Evidence;
use std::collections::BTreeSet;

pub fn judge(evidence: &Evidence) -> Found {
    let want = &evidence.expected;
    let policy = Policy(&evidence.actual);
    let mut found = Vec::new();
    compare(
        "scan include",
        policy.list("scan", "include"),
        want.include.clone(),
        &mut found,
    );
    compare(
        "scan exclude",
        policy.list("scan", "exclude"),
        want.exclude.clone(),
        &mut found,
    );
    found.extend(either(
        "module roots",
        policy.list("module", "roots"),
        want.roots.clone(),
        evidence.factory.roots.clone(),
    ));
    policy.limits(&mut found);
    policy.setting(
        ("comment", "allow"),
        crate::catalog::set::setting("comment", "allow"),
        &mut found,
    );
    policy.setting(
        ("word", "single"),
        crate::catalog::set::setting("word", "single"),
        &mut found,
    );
    compare(
        "test grant",
        policy.syntax("grant", "test"),
        want.tests.clone(),
        &mut found,
    );
    require(
        "environment grant",
        policy.syntax("grant", "environment"),
        want.tests.clone(),
        &mut found,
    );
    require(
        "style ban",
        policy.syntax("ban", "style"),
        want.bans.clone(),
        &mut found,
    );
    found
        .into_iter()
        .map(|line| wrong(&rule::ECTROPY_POLICY, line))
        .collect()
}

struct Policy<'a>(&'a toml::Value);

impl Policy<'_> {
    fn limits(&self, found: &mut Vec<String>) {
        let factory = crate::catalog::set::factory();
        for (name, value) in crate::catalog::set::limits() {
            let seen = self
                .0
                .get("limit")
                .and_then(|table| table.get(&name))
                .and_then(toml::Value::as_integer);
            if seen != Some(value) && seen != factory.get(&name).copied() {
                found.push(format!("ectropy limit {name} must be {value}"));
            }
        }
    }

    fn setting(&self, key: (&str, &str), want: bool, found: &mut Vec<String>) {
        let seen = self
            .0
            .get(key.0)
            .and_then(|value| value.get(key.1))
            .and_then(toml::Value::as_bool);
        if seen != Some(want) {
            found.push(format!("ectropy {}.{} must be {want}", key.0, key.1));
        }
    }

    fn list(&self, table: &str, key: &str) -> BTreeSet<String> {
        self.0
            .get(table)
            .and_then(|value| value.get(key))
            .and_then(toml::Value::as_array)
            .map(|values| strings(values))
            .unwrap_or_default()
    }

    fn syntax(&self, table: &str, name: &str) -> BTreeSet<String> {
        let Some(entries) = self.0.get(table).and_then(toml::Value::as_array) else {
            return BTreeSet::new();
        };
        entries
            .iter()
            .filter(|entry| entry.get("syntax").and_then(toml::Value::as_str) == Some(name))
            .filter_map(|entry| entry.get("paths").and_then(toml::Value::as_array))
            .flat_map(|values| strings(values))
            .collect()
    }
}

fn strings(values: &[toml::Value]) -> BTreeSet<String> {
    values
        .iter()
        .filter_map(toml::Value::as_str)
        .map(str::to_string)
        .collect()
}

fn compare(name: &str, have: BTreeSet<String>, want: BTreeSet<String>, found: &mut Vec<String>) {
    require(name, have.clone(), want.clone(), found);
    for value in have.difference(&want) {
        found.push(format!("unexpected ectropy {name} {value}"));
    }
}

fn either(
    name: &str,
    have: BTreeSet<String>,
    want: BTreeSet<String>,
    factory: BTreeSet<String>,
) -> Vec<String> {
    if have == want || have == factory {
        return Vec::new();
    }
    let mut found = Vec::new();
    require(name, have.clone(), want.clone(), &mut found);
    for value in have.difference(&want) {
        found.push(format!("unexpected ectropy {name} {value}"));
    }
    found
}

fn require(name: &str, have: BTreeSet<String>, want: BTreeSet<String>, found: &mut Vec<String>) {
    for value in want.difference(&have) {
        found.push(format!("missing ectropy {name} {value}"));
    }
}
