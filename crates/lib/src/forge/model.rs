use serde_json::Value;

#[derive(Clone)]
pub struct Remote {
    pub scheme: String,
    pub host: String,
    pub owner: String,
    pub repo: String,
}

pub struct Pull {
    pub number: u64,
    pub url: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Strategy {
    Merge,
    Forward,
}

impl Strategy {
    pub(super) fn wire(self) -> &'static str {
        match self {
            Self::Merge => "merge",
            Self::Forward => "fast-forward-only",
        }
    }
}

pub struct State {
    pub state: String,
    pub count: usize,
}

impl State {
    pub fn read(value: &Value) -> Self {
        Self {
            state: value
                .get("state")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            count: value
                .get("statuses")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or_default(),
        }
    }
}
