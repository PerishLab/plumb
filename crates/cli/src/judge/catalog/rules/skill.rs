use super::{Rule, rule};

rule!(
    TEXT_BUDGET,
    "skill.text-budget",
    "Skill text stays bounded by source scale",
    "A skill remains a compact operating brief whose text budget grows sublinearly with the production source it explains.",
    "Doctor's production source lines, skill Markdown lines, file count, and derived budget.",
    Observed,
    SKILL,
    [REPOSITORY, SKILL_TAG]
);

pub fn all() -> Vec<&'static Rule> {
    vec![&TEXT_BUDGET]
}
