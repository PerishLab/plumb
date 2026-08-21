use serde::Serialize;

pub struct Owner {
    pub id: String,
    pub summary: String,
}

pub struct Tag {
    pub id: String,
    pub summary: String,
}

pub struct Namespace {
    pub id: String,
    pub summary: String,
    pub owner: String,
}

pub fn owners() -> &'static [Owner] {
    &super::held().owners
}

pub fn tags() -> &'static [Tag] {
    &super::held().tags
}

pub fn namespaces() -> &'static [Namespace] {
    &super::held().namespaces
}

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
            id: &owner.id,
            summary: &owner.summary,
        }
    }
}

impl From<&'static Tag> for Label {
    fn from(tag: &'static Tag) -> Self {
        Self {
            id: &tag.id,
            summary: &tag.summary,
        }
    }
}

impl From<&'static Namespace> for Scope {
    fn from(namespace: &'static Namespace) -> Self {
        Self {
            id: &namespace.id,
            summary: &namespace.summary,
            owner: &namespace.owner,
        }
    }
}
