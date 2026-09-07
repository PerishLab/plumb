use crate::shape::layout::{Group, Held};
use plumb::rule::Probe;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn programs(probes: &BTreeMap<String, Vec<Probe>>) -> Result<Vec<String>, String> {
    probes
        .values()
        .map(|rules| {
            Probe::select(rules, &plumb::config::platform()).map(|probe| probe.argv[0].clone())
        })
        .collect()
}

pub(crate) fn covers(probes: &BTreeMap<String, Vec<Probe>>, argv: &[&str]) -> Result<bool, String> {
    for rules in probes.values() {
        if Probe::select(rules, &plumb::config::platform())?.argv == argv {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(crate) fn evidence(
    probes: &BTreeMap<String, Vec<Probe>>,
    execution: &plumb::config::Execution,
) -> Result<Vec<u8>, String> {
    let mut observations = BTreeMap::new();
    for (name, rules) in probes {
        let probe = Probe::select(rules, &plumb::config::platform())?;
        let observed = probe.run(execution)?;
        if !observed.matches {
            return Err(format!(
                "{name} probe {:?} expected stdout {:?}, observed {:?}",
                probe.argv, probe.stdout, observed.stdout
            ));
        }
        observations.insert(name, (probe, observed));
    }
    serde_json::to_vec(&observations)
        .map_err(|error| format!("cannot encode probe evidence: {error}"))
}

pub(crate) fn read<'a>(
    manifest: &str,
    paths: impl IntoIterator<Item = &'a str>,
) -> Result<BTreeMap<String, Vec<Probe>>, String> {
    let declared = match crate::shape::layout::parse(manifest) {
        Held::Stated(declared) => declared,
        Held::Wrong(error) => return Err(error),
        _ => return Ok(BTreeMap::new()),
    };
    let paths = paths.into_iter().collect::<BTreeSet<_>>();
    let mut probes = BTreeMap::new();
    for group in declared
        .groups
        .iter()
        .filter(|group| applies(group, &paths))
    {
        for name in &group.rule {
            if probes.contains_key(name) {
                continue;
            }
            let reference = super::member::parse(name)?;
            let member = super::member::member(&reference)?;
            if !member.probe.is_empty() {
                probes.insert(name.clone(), member.probe);
            }
        }
    }
    Ok(probes)
}

pub(crate) fn applies(group: &Group, paths: &BTreeSet<&str>) -> bool {
    !group.retired && group.names.iter().any(|name| paths.contains(name.as_str()))
}
