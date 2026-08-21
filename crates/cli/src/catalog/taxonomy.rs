use serde::Serialize;
use std::sync::LazyLock;

pub struct Owner {
    pub id: &'static str,
    pub summary: &'static str,
}

pub struct Tag {
    pub id: &'static str,
    pub summary: &'static str,
}

pub struct Namespace {
    pub id: &'static str,
    pub summary: &'static str,
    pub owner: &'static Owner,
}

pub static OWNERS: LazyLock<Vec<&'static Owner>> = LazyLock::new(|| super::held().owners.clone());

pub static TAGS: LazyLock<Vec<&'static Tag>> = LazyLock::new(|| super::held().tags.clone());

pub static NAMESPACES: LazyLock<Vec<&'static Namespace>> =
    LazyLock::new(|| super::held().namespaces.clone());

#[derive(Serialize)]
pub struct Ownership {
    pub id: &'static str,
    pub summary: &'static str,
}

#[derive(Serialize)]
pub struct Label {
    pub id: &'static str,
    pub summary: &'static str,
}

#[derive(Serialize)]
pub struct Scope {
    pub id: &'static str,
    pub summary: &'static str,
    pub owner: &'static str,
}

impl From<&'static Owner> for Ownership {
    fn from(owner: &'static Owner) -> Self {
        Self {
            id: owner.id,
            summary: owner.summary,
        }
    }
}

impl From<&'static Tag> for Label {
    fn from(tag: &'static Tag) -> Self {
        Self {
            id: tag.id,
            summary: tag.summary,
        }
    }
}

impl From<&'static Namespace> for Scope {
    fn from(namespace: &'static Namespace) -> Self {
        Self {
            id: namespace.id,
            summary: namespace.summary,
            owner: namespace.owner.id,
        }
    }
}
