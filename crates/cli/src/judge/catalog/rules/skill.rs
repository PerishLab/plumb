use super::{Rule, rule};

rule!(
    TEXT_BUDGET,
    "skill.text-budget",
    "Skill text stays bounded by source scale",
    "A skill remains a compact operating brief whose text budget grows sublinearly with the production source it explains.",
    "The selected limit strategy, readable production source lines, readable skill Markdown lines, and its derived aggregate budget.",
    Mechanized,
    SKILL,
    [REPOSITORY, SKILL_TAG]
);

rule!(
    LIMIT_STRATEGY,
    "skill.limit-strategy",
    "Skill shape selects one closed limit strategy",
    "A repository with skill seats explicitly selects one Plumb-owned strategy that closes both their root file enumeration and text magnitude without exposing either as parameters.",
    "The skill strategy in plumb.toml and the exact regular root entries in each skill seat, without following symbolic links.",
    Mechanized,
    SKILL,
    [REPOSITORY, SKILL_TAG]
);

pub fn all() -> Vec<&'static Rule> {
    vec![&TEXT_BUDGET, &LIMIT_STRATEGY]
}
