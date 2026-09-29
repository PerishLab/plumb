use std::fmt;
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Expectation {
    pub branch: String,
    pub tracking: String,
    pub target: String,
}

impl Expectation {
    pub fn new(
        branch: impl Into<String>,
        tracking: impl Into<String>,
        target: impl Into<String>,
    ) -> Self {
        Self {
            branch: branch.into(),
            tracking: tracking.into(),
            target: target.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Relation {
    Equal,
    Behind,
    Ahead,
    Diverged,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Checkout {
    pub path: PathBuf,
    pub common: PathBuf,
    pub branch: Option<String>,
    pub head: String,
    pub tree: String,
    pub clean: bool,
    pub upstream: Option<String>,
    pub tracked: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Worktree {
    pub path: PathBuf,
    pub branch: Option<String>,
    pub head: Option<String>,
    pub bare: bool,
    pub detached: bool,
    pub locked: bool,
    pub prunable: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Inspection {
    pub expected: Expectation,
    pub tree: String,
    pub checkout: Checkout,
    pub relation: Relation,
    pub worktrees: Vec<Worktree>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Refusal {
    pub code: &'static str,
    pub message: String,
    pub inspection: Option<Box<Inspection>>,
}

impl Refusal {
    pub(crate) fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            inspection: None,
        }
    }

    pub(crate) fn observed(
        code: &'static str,
        message: impl Into<String>,
        inspection: Inspection,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            inspection: Some(Box::new(inspection)),
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for Refusal {}
