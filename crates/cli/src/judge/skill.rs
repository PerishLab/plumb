use super::catalog::rules::skill as rule;
use super::finding::{Found, blind, wrong};
use crate::shape;

pub fn check(held: &shape::Shape) -> Found {
    use shape::skill::Config;

    let mut found = Found::new();
    let strategy = match &held.skills.config {
        Config::Outside => return found,
        Config::Absent if !held.skills.present => return found,
        Config::Absent => {
            found.push(wrong(
                &rule::LIMIT_STRATEGY,
                "skills exist but plumb.toml declares no skill strategy",
            ));
            return found;
        }
        Config::Wrong(error) => {
            found.push(wrong(&rule::LIMIT_STRATEGY, error));
            return found;
        }
        Config::Blind(error) => {
            found.push(blind(&rule::LIMIT_STRATEGY, error));
            return found;
        }
        Config::Held(strategy) => *strategy,
    };
    if !held.skills.present || held.skills.held.is_empty() {
        found.push(wrong(
            &rule::LIMIT_STRATEGY,
            format!("skill strategy {} has no skill seat", strategy.id()),
        ));
        return found;
    }
    if let Some(error) = &held.skills.unread {
        let evidence = format!(
            "cannot inspect skills: {}",
            error.lines().next().unwrap_or("")
        );
        found.push(blind(&rule::LIMIT_STRATEGY, evidence.clone()));
        found.push(blind(&rule::TEXT_BUDGET, evidence));
    }
    for skill in &held.skills.held {
        shape(strategy, skill, &mut found);
        budget(skill, &mut found);
    }
    found
}

fn shape(strategy: shape::skill::Strategy, skill: &shape::skill::Skill, found: &mut Found) {
    if let Some(error) = skill.unread.form.first() {
        found.push(blind(
            &rule::LIMIT_STRATEGY,
            format!("skill {} shape is unreadable: {error}", skill.name),
        ));
        return;
    }
    if !skill.seat {
        found.push(wrong(
            &rule::LIMIT_STRATEGY,
            format!("skill {} is not a regular directory", skill.name),
        ));
        return;
    }
    let documents = strategy.documents();
    let missing = documents
        .iter()
        .filter(|name| !skill.regular.iter().any(|held| held == **name))
        .copied()
        .collect::<Vec<_>>();
    let extra = skill
        .entries
        .iter()
        .filter(|name| !documents.contains(&name.as_str()))
        .map(String::as_str)
        .collect::<Vec<_>>();
    if !missing.is_empty() || !extra.is_empty() {
        found.push(wrong(
            &rule::LIMIT_STRATEGY,
            format!(
                "skill {} under strategy {} has missing regular files [{}] and extra root entries [{}]",
                skill.name,
                strategy.id(),
                missing.join(", "),
                extra.join(", ")
            ),
        ));
    }
}

fn budget(skill: &shape::skill::Skill, found: &mut Found) {
    let unread = skill.unread.source.iter().chain(&skill.unread.text).next();
    if let Some(error) = unread {
        found.push(blind(
            &rule::TEXT_BUDGET,
            format!("skill {} text budget is unreadable: {error}", skill.name),
        ));
        return;
    }
    let Some(budget) = skill.budget else {
        return;
    };
    if skill.text > budget {
        found.push(wrong(
            &rule::TEXT_BUDGET,
            format!(
                "skill {} has {} Markdown lines, above budget {} for {} source lines",
                skill.name, skill.text, budget, skill.source
            ),
        ));
    }
}
