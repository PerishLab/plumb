use super::{Rule, rule};

rule!(
    TEXT_BUDGET,
    "skill.text-budget",
    "Skill text stays bounded by source scale",
    "A skill remains a compact operating brief whose text budget grows sublinearly with the production source it explains.",
    "Readable production source lines, readable skill Markdown lines, and the derived aggregate budget.",
    Mechanized,
    SKILL,
    [REPOSITORY, SKILL_TAG]
);

rule!(
    THREE_PART,
    "skill.three-part",
    "Skills use one fixed three-part brief",
    "A skill contains exactly SKILL.md for objects and actions, PATHS.md for hot paths, and SCENARIOS.md for restrained complex scenarios.",
    "The exact three regular root entries in each repository skill seat, without following symbolic links.",
    Mechanized,
    SKILL,
    [REPOSITORY, SKILL_TAG]
);

pub fn all() -> Vec<&'static Rule> {
    vec![&TEXT_BUDGET, &THREE_PART]
}
