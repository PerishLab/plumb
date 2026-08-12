use super::{Rule, rule};

rule!(
    SCHEMA,
    "document.closed-schema",
    "Documents select one closed Plumb strategy",
    "A governed document declares only a Plumb-owned strategy, its objective source seats, and explicit human affirmation evidence.",
    "The document tables in plumb.toml and the exact target shape derived from each selected strategy.",
    Mechanized,
    PLUMB,
    [DOCUMENT_TAG, REPOSITORY]
);

rule!(
    ADMISSION,
    "document.closed-surface",
    "Tracked prose stays inside the document surface",
    "Repository prose occupies only document seats admitted by the current Plumb release.",
    "The exact Git index and the target paths derived from document strategies.",
    Mechanized,
    PLUMB,
    [DOCUMENT_TAG, REPOSITORY]
);

rule!(
    EVIDENCE,
    "document.evidence-current",
    "Document affirmations match both sides",
    "A current projection is reread whenever its source bytes, target bytes, or binding topology changes.",
    "Canonical source seals, target seals, and document binding topology in plumb.toml.",
    Mechanized,
    PLUMB,
    [DOCUMENT_TAG, OWNERSHIP]
);

rule!(
    MAGNITUDE,
    "document.magnitude",
    "Document text stays within its strategy magnitude",
    "Every document strategy applies its own Plumb-owned text magnitude to the exact target and evidence mass.",
    "Tracked target text and the strategy-specific fixed, source-line, or source-leaf magnitude.",
    Mechanized,
    PLUMB,
    [DOCUMENT_TAG, REPOSITORY]
);

pub fn all() -> Vec<&'static Rule> {
    vec![&ADMISSION, &EVIDENCE, &MAGNITUDE, &SCHEMA]
}
