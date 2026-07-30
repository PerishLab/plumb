use super::catalog::rules::deps as rule;
use super::finding::{blind, wrong};
use crate::rules::RULES;
use crate::shape::{self, Found};
use semver::{Version, VersionReq};

pub fn check(held: &shape::Shape) -> Found {
    let mut found = Found::new();
    if held.binary && !held.clap {
        found.push(wrong(
            &rule::RUST_BINARY_USES_CLAP,
            "ships a rust binary without clap",
        ));
    }
    if held.binary && !held.substrate {
        found.push(wrong(
            &rule::RUST_BINARY_USES_PLUMB,
            "ships a rust binary without plumb",
        ));
    }
    for (name, current) in &RULES.retired {
        if held.deno.contains(name.as_str()) {
            found.push(wrong(
                &rule::CURRENT_DEPENDENCY_NAME,
                format!("depends on {name}, renamed to {current}"),
            ));
        }
    }
    sealkit(held, &mut found);
    for name in RULES.pinned(&held.deno) {
        found.push(wrong(
            &rule::SELF_BUILT_DEPENDENCY_UNPINNED,
            format!("self-built {name} is version-pinned, the skeleton tracks latest"),
        ));
    }
    for (seat, name) in &held.node {
        if RULES.blacklist.contains(name) {
            found.push(wrong(
                &rule::STYLING_PACKAGE_ALLOWED,
                format!("{seat} depends on blacklisted styling package {name}"),
            ));
        }
    }
    found
}

fn sealkit(held: &shape::Shape, found: &mut Found) {
    let Some(support) = RULES.support("@perish/sealkit") else {
        return;
    };
    let dependency = match &held.sealkit {
        shape::Sealkit::Absent => return,
        shape::Sealkit::Blind(error) => {
            found.push(blind(&rule::SUPPORTED_DEPENDENCY_LINE, error.clone()));
            return;
        }
        shape::Sealkit::Held(dependency) => dependency,
    };
    if dependency.requirement != support.legacy && dependency.requirement != support.requirement {
        found.push(wrong(
            &rule::SUPPORTED_DEPENDENCY_LINE,
            format!(
                "unsupported Sealkit requirement {}; use {} or {}",
                dependency.requirement, support.legacy, support.requirement
            ),
        ));
        return;
    }
    let Ok(resolution) = Version::parse(&dependency.resolution) else {
        refuse(&dependency.resolution, support, found);
        return;
    };
    let minimum =
        Version::parse(&support.minimum).expect("support minimum must be a semantic version");
    if dependency.requirement == support.requirement && resolution < minimum {
        found.push(wrong(
            &rule::SUPPORTED_DEPENDENCY_LINE,
            format!("Sealkit resolution {resolution} is below supported Sealkit floor {minimum}"),
        ));
        return;
    }
    let admitted = VersionReq::parse(&support.requirement)
        .expect("support requirement must be a semantic version requirement")
        .matches(&resolution)
        || precedent(&support.line, &resolution);
    if !admitted {
        refuse(&resolution.to_string(), support, found);
    }
}

fn refuse(resolution: &str, support: &crate::rules::Support, found: &mut Found) {
    found.push(wrong(
        &rule::SUPPORTED_DEPENDENCY_LINE,
        format!(
            "unsupported Sealkit resolution {resolution}; transition admits {} or {}",
            support.line, support.requirement
        ),
    ));
}

fn precedent(line: &str, version: &Version) -> bool {
    let mut parts = line.split('.');
    let major = parts.next().and_then(|part| part.parse::<u64>().ok());
    let minor = parts.next().and_then(|part| part.parse::<u64>().ok());
    parts.next().is_none()
        && major == Some(version.major)
        && minor == Some(version.minor)
        && version.pre.is_empty()
}
