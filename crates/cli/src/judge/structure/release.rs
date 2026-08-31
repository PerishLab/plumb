use crate::catalog::rules::release as release_rule;
use crate::catalog::rules::structure as rule;
use crate::judge::finding::{Found, Seed};
use crate::shape;
use crate::shape::pair::Release;

pub fn judge(held: &shape::Shape) -> Found {
    Judge(held).run()
}

struct Judge<'a>(&'a shape::Shape);

impl Judge<'_> {
    fn run(&self) -> Found {
        let held = self.0;
        let mut found = Found::new();
        if held.rust && !held.ignore.lines().any(|line| line.trim() == "target/") {
            found.push(Seed::wrong(
                &rule::CARGO_TARGET_IGNORED,
                "Cargo.toml without target/ in .gitignore".to_string(),
            ));
        }
        spec(&held.release, &mut found);
        measured(&held.release, &mut found);
        found
    }
}

fn spec(release: &Release, found: &mut Found) {
    if let Some(refusal) = &release.refusal {
        found.push(Seed::wrong(
            &release_rule::SPEC_DECLARED,
            format!("plumb.toml declares a release the current Plumb refuses: {refusal}"),
        ));
    }
    if let Some(blind) = &release.blind {
        found.push(Seed::blind(&release_rule::SPEC_DECLARED, blind));
    }
}

fn measured(release: &Release, found: &mut Found) {
    let rules = &crate::catalog::set::current().release;
    for (attachment, held) in &release.widths {
        let permitted = rules
            .permitted
            .get(attachment)
            .copied()
            .unwrap_or(rules.ceiling);
        if *held > permitted {
            found.push(Seed::wrong(
                &release_rule::ATTACHMENT_PERMITTED,
                format!(
                    "the {attachment} attachment declares {held} packages and Plumb permits {permitted}; widening it is a change to Plumb"
                ),
            ));
            continue;
        }
        let exercised = rules.exercised.get(attachment).copied().unwrap_or(0);
        if *held > exercised {
            found.push(Seed::noted(
                &release_rule::ATTACHMENT_EXERCISED,
                format!(
                    "the {attachment} attachment declares {held} packages and the skeleton has released {exercised}; this width has never run"
                ),
            ));
        }
    }
}
