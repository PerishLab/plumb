use super::catalog::rules::skill as rule;
use super::finding::{Found, blind, wrong};
use crate::shape;

pub fn check(held: &shape::Shape) -> Found {
    let mut found = Found::new();
    if let Some(error) = &held.skills.unread {
        let evidence = format!(
            "cannot inspect skills: {}",
            error.lines().next().unwrap_or("")
        );
        found.push(blind(&rule::THREE_PART, evidence.clone()));
        found.push(blind(&rule::TEXT_BUDGET, evidence));
    }
    for skill in &held.skills.held {
        shape(skill, &mut found);
        budget(skill, &mut found);
    }
    found
}

fn shape(skill: &shape::skill::Skill, found: &mut Found) {
    if let Some(error) = skill.unread.form.first() {
        found.push(blind(
            &rule::THREE_PART,
            format!("skill {} shape is unreadable: {error}", skill.name),
        ));
        return;
    }
    if !skill.seat {
        found.push(wrong(
            &rule::THREE_PART,
            format!("skill {} is not a regular directory", skill.name),
        ));
        return;
    }
    let missing = shape::skill::DOCUMENTS
        .iter()
        .filter(|name| !skill.regular.iter().any(|held| held == **name))
        .copied()
        .collect::<Vec<_>>();
    let extra = skill
        .entries
        .iter()
        .filter(|name| !shape::skill::DOCUMENTS.contains(&name.as_str()))
        .map(String::as_str)
        .collect::<Vec<_>>();
    if !missing.is_empty() || !extra.is_empty() {
        found.push(wrong(
            &rule::THREE_PART,
            format!(
                "skill {} has missing regular files [{}] and extra root entries [{}]",
                skill.name,
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
    } else if skill.text > skill.budget {
        found.push(wrong(
            &rule::TEXT_BUDGET,
            format!(
                "skill {} has {} Markdown lines, above budget {} for {} source lines",
                skill.name, skill.text, skill.budget, skill.source
            ),
        ));
    }
}
