pub(crate) mod affirm;
mod manifest;
mod read;

pub use read::{parse, read, stated};

pub struct Read {
    pub held: Held,
    pub(crate) evidence: Option<Evidence>,
}

pub(crate) struct Evidence {
    pub snapshot: plumb::snapshot::Snapshot,
    pub repository: String,
    pub affirmed: affirm::Held,
}

#[derive(Clone)]
pub enum Held {
    Outside,
    Absent,
    Wrong(String),
    Stated(Declared),
}

#[derive(Clone, Default)]
pub struct Declared {
    pub seats: Vec<Seat>,
    pub groups: Vec<Group>,
    pub overrides: Vec<String>,
}

#[derive(Clone)]
pub struct Seat {
    pub path: String,
    pub anchor: Option<Vec<String>>,
    pub rule: Vec<String>,
    pub retired: bool,
    pub depot: Option<String>,
}

#[derive(Clone)]
pub struct Group {
    pub names: Vec<String>,
    pub rule: Vec<String>,
    pub retired: bool,
}

impl Seat {
    pub fn container(&self) -> Option<&str> {
        self.path.strip_suffix("/*")
    }

    pub fn head(&self) -> &str {
        self.path.split('/').next().unwrap_or(&self.path)
    }

    pub fn shown(&self) -> String {
        let anchor = match &self.anchor {
            None => String::new(),
            Some(held) => format!("/{{{}}}", held.join(", ")),
        };
        format!("{}{}{}", self.path, anchor, shown(&self.rule))
    }
}

impl Group {
    pub fn shown(&self) -> String {
        format!("{{{}}}{}", self.names.join(", "), shown(&self.rule))
    }
}

fn shown(rule: &[String]) -> String {
    if rule.is_empty() {
        return String::new();
    }
    let held = rule
        .iter()
        .map(|name| format!("\"{name}\""))
        .collect::<Vec<_>>()
        .join(", ");
    format!("#[{held}]")
}
