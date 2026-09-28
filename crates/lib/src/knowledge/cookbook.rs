use serde::Serialize;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct Code(String);

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    code: Code,
    trigger: String,
    solution: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    evidence: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Cookbook {
    entries: Vec<Entry>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    Code(String),
    Blank { code: String, field: &'static str },
    Duplicate(String),
}

impl Code {
    pub fn new(raw: impl Into<String>) -> Result<Self, Error> {
        let raw = raw.into();
        if valid(&raw) {
            Ok(Self(raw))
        } else {
            Err(Error::Code(raw))
        }
    }

    pub fn text(&self) -> &str {
        &self.0
    }
}

impl Display for Code {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Entry {
    pub fn new(
        code: impl Into<String>,
        trigger: impl Into<String>,
        solution: impl Into<String>,
    ) -> Result<Self, Error> {
        let code = Code::new(code)?;
        let trigger = required(&code, "trigger", trigger.into())?;
        let solution = required(&code, "solution", solution.into())?;
        Ok(Self {
            code,
            trigger,
            solution,
            evidence: None,
            exit: None,
        })
    }

    pub fn observe(mut self, text: impl Into<String>) -> Result<Self, Error> {
        self.evidence = Some(required(&self.code, "evidence", text.into())?);
        Ok(self)
    }

    pub fn retire(mut self, text: impl Into<String>) -> Result<Self, Error> {
        self.exit = Some(required(&self.code, "exit", text.into())?);
        Ok(self)
    }

    pub fn code(&self) -> &Code {
        &self.code
    }

    pub fn trigger(&self) -> &str {
        &self.trigger
    }

    pub fn solution(&self) -> &str {
        &self.solution
    }

    pub fn evidence(&self) -> Option<&str> {
        self.evidence.as_deref()
    }

    pub fn exit(&self) -> Option<&str> {
        self.exit.as_deref()
    }
}

impl Cookbook {
    pub fn new(entries: impl IntoIterator<Item = Entry>) -> Result<Self, Error> {
        let mut entries = entries.into_iter().collect::<Vec<_>>();
        entries.sort_by(|held, other| held.code.cmp(&other.code));
        for pair in entries.windows(2) {
            if pair[0].code == pair[1].code {
                return Err(Error::Duplicate(pair[0].code.to_string()));
            }
        }
        Ok(Self { entries })
    }

    pub fn get(&self, code: &str) -> Option<&Entry> {
        self.entries
            .binary_search_by(|entry| entry.code.text().cmp(code))
            .ok()
            .map(|index| &self.entries[index])
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Code(code) => write!(formatter, "invalid Cookbook code `{code}`"),
            Self::Blank { code, field } => {
                write!(formatter, "Cookbook entry `{code}` has blank {field}")
            }
            Self::Duplicate(code) => write!(formatter, "duplicate Cookbook code `{code}`"),
        }
    }
}

impl std::error::Error for Error {}

fn required(code: &Code, field: &'static str, text: String) -> Result<String, Error> {
    if text.trim().is_empty() {
        Err(Error::Blank {
            code: code.to_string(),
            field,
        })
    } else {
        Ok(text)
    }
}

fn valid(code: &str) -> bool {
    let mut parts = code.split('.');
    let Some(namespace) = parts.next() else {
        return false;
    };
    let Some(first) = parts.next() else {
        return false;
    };
    segment(namespace) && segment(first) && parts.all(segment)
}

fn segment(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some(first) if first.is_ascii_lowercase())
        && chars.all(|held| matches!(held, 'a'..='z' | '0'..='9' | '-' | '_'))
}
