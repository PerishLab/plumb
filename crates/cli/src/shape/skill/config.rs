use std::path::Path;

const BRIEF: [&str; 3] = ["PATHS.md", "SCENARIOS.md", "SKILL.md"];

#[derive(Clone)]
pub enum Config {
    Outside,
    Absent,
    Held(Strategy),
    Wrong(String),
    Blind(String),
}

#[derive(Clone, Copy)]
pub enum Strategy {
    Brief,
}

impl Config {
    pub fn strategy(&self) -> Option<Strategy> {
        match self {
            Self::Held(strategy) => Some(*strategy),
            _ => None,
        }
    }
}

impl Strategy {
    pub fn id(self) -> &'static str {
        match self {
            Self::Brief => "brief",
        }
    }

    pub fn documents(self) -> &'static [&'static str] {
        match self {
            Self::Brief => &BRIEF,
        }
    }

    pub fn budget(self, source: usize) -> usize {
        match self {
            Self::Brief => {
                let root = (source as f64).sqrt().ceil() as usize;
                (root * 2).clamp(120, 400)
            }
        }
    }
}

pub fn read(root: &Path) -> Config {
    let path = root.join("plumb.toml");
    if !path.exists() {
        return Config::Outside;
    }
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => return Config::Blind(error.to_string()),
    };
    let doc = match text.parse::<toml::Table>() {
        Ok(doc) => doc,
        Err(error) => return Config::Wrong(format!("cannot parse plumb.toml: {error}")),
    };
    let Some(value) = doc.get("skill") else {
        return Config::Absent;
    };
    let Some(table) = value.as_table() else {
        return Config::Wrong("skill must be a table".to_string());
    };
    let extra = table
        .keys()
        .filter(|key| key.as_str() != "strategy")
        .map(String::as_str)
        .collect::<Vec<_>>();
    if !extra.is_empty() {
        return Config::Wrong(format!("skill has unknown fields: {}", extra.join(", ")));
    }
    let Some(value) = table.get("strategy") else {
        return Config::Wrong("skill misses strategy".to_string());
    };
    let Some(value) = value.as_str() else {
        return Config::Wrong("skill strategy must be a string".to_string());
    };
    match value {
        "brief" => Config::Held(Strategy::Brief),
        _ => Config::Wrong(format!("unknown skill strategy {value}")),
    }
}
