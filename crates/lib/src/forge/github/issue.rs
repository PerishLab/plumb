use serde::Serialize;
use serde_json::Value;

pub const FIELDS: &str =
    "id,number,url,title,state,updatedAt,issueType,parent,subIssues,blockedBy,blocking";

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Issue {
    pub node: String,
    pub number: u64,
    pub url: String,
    pub title: String,
    pub state: String,
    pub kind: String,
    pub updated: String,
    pub parent: Option<Reference>,
    #[serde(rename = "sub_issues")]
    pub parts: Vec<Reference>,
    #[serde(rename = "blocked_by")]
    pub behind: Vec<Reference>,
    pub blocking: Vec<Reference>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Reference {
    pub repository: String,
    pub number: u64,
    pub node: String,
    pub url: String,
    pub title: String,
    pub state: String,
}

pub fn parse(value: &Value) -> Option<Issue> {
    Shape(value).issue()
}

struct Shape<'a>(&'a Value);

impl Shape<'_> {
    fn issue(&self) -> Option<Issue> {
        let mut parts = self.many("subIssues");
        let mut behind = self.many("blockedBy");
        let mut blocking = self.many("blocking");
        parts.sort();
        behind.sort();
        blocking.sort();
        Some(Issue {
            node: self.text("id"),
            number: self.0.get("number")?.as_u64()?,
            url: self.text("url"),
            title: self.text("title"),
            state: self.text("state"),
            kind: self
                .0
                .get("issueType")
                .and_then(|kind| kind.get("name"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            updated: self.text("updatedAt"),
            parent: self
                .0
                .get("parent")
                .and_then(|value| Shape(value).reference()),
            parts,
            behind,
            blocking,
        })
    }

    fn many(&self, key: &str) -> Vec<Reference> {
        self.0
            .get(key)
            .and_then(|relation| relation.get("nodes"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|value| Shape(value).reference())
            .collect()
    }

    fn reference(&self) -> Option<Reference> {
        let url = self.0.get("url")?.as_str()?.to_string();
        let repository = url
            .strip_prefix("https://github.com/")?
            .split_once("/issues/")?
            .0
            .to_string();
        Some(Reference {
            repository,
            number: self.0.get("number")?.as_u64()?,
            node: self.text("id"),
            url,
            title: self.text("title"),
            state: self.text("state"),
        })
    }

    fn text(&self, key: &str) -> String {
        self.0
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn issues() {
        let value = serde_json::json!({
            "id": "I_one",
            "number": 20,
            "url": "https://github.com/PerishLab/plumb/issues/20",
            "title": "Plan delivery",
            "state": "OPEN",
            "updatedAt": "2026-09-26T00:00:00Z",
            "issueType": { "name": "Feature" },
            "parent": null,
            "subIssues": { "nodes": [] },
            "blockedBy": { "nodes": [{
                "id": "I_root",
                "number": 1,
                "url": "https://github.com/PerishLab/.github/issues/1",
                "title": "Contract",
                "state": "CLOSED"
            }] },
            "blocking": { "nodes": [] }
        });
        let issue = parse(&value).expect("issue");
        assert_eq!(issue.kind, "Feature");
        assert_eq!(issue.behind[0].repository, "PerishLab/.github");
    }
}
