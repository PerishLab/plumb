use super::{Definition, Judgment, Reference, wire::Wire};
use crate::windtun::qualification::{Fault, Read};
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Author {
    pub id: u64,
    pub login: String,
    pub permission: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Review<T = Judgment> {
    pub reference: Reference,
    pub judgment: T,
    pub author: Author,
    pub comment: String,
    pub repository: Value,
}

pub(super) struct Reader<'a, R>(pub &'a R);

impl<R: Read> Reader<'_, R> {
    fn coordinate<'a>(&self, location: &'a str) -> Result<(&'a str, &'a str, &'a str), Fault> {
        super::super::wire::Wire::reference(location).map_err(Fault::refused)?;
        let (location, comment) = location.split_once("#issuecomment-").unwrap();
        let path = location
            .strip_prefix("https://github.com/PerishLab/")
            .unwrap();
        let parts = path.split('/').collect::<Vec<_>>();
        Ok((parts[0], parts[2], comment))
    }

    fn author(&self, product: &str, value: &Value) -> Result<Author, Fault> {
        Wire::require(
            value["type"] == "User",
            "semantic review author kind is unsupported",
        )?;
        let id = value["id"]
            .as_u64()
            .filter(|id| *id > 0)
            .ok_or_else(|| Fault::refused("semantic review author has no native identity"))?;
        let login = Wire::text(&value["login"])?;
        Wire::require(
            login.len() <= 39
                && login
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-'),
            "semantic review author login is invalid",
        )?;
        let permission: Value = Wire::document(
            self.0,
            &format!(
                "https://api.github.com/repos/PerishLab/{product}/collaborators/{login}/permission"
            ),
        )?;
        Wire::require(
            permission["user"]["id"] == id
                && permission["user"]["login"] == login
                && permission["user"]["type"] == "User",
            "semantic review permission names another author",
        )?;
        let role = Wire::text(&permission["permission"])?;
        Wire::require(
            ["admin", "write"].contains(&role),
            "semantic review author lacks current product write permission",
        )?;
        Ok(Author {
            id,
            login: login.into(),
            permission: role.into(),
        })
    }

    pub fn inspect(&self, location: &str, product: &str) -> Result<Review, Fault> {
        self.document((location, product), Judgment::parse)
    }

    pub fn adoption(&self, location: &str, product: &str) -> Result<Review<Definition>, Fault> {
        self.document((location, product), Definition::parse)
    }

    fn document<T: Serialize>(
        &self,
        selected: (&str, &str),
        parse: fn(&str) -> Result<T, Fault>,
    ) -> Result<Review<T>, Fault> {
        let (location, product) = selected;
        Wire::name(product)?;
        let repository = Wire::repository(self.0, &format!("PerishLab/{product}"))?;
        let (origin, issue, number) = self.coordinate(location)?;
        let comments = Wire::repository(self.0, &format!("PerishLab/{origin}"))?;
        let url =
            format!("https://api.github.com/repos/PerishLab/{origin}/issues/comments/{number}");
        let value: Value = Wire::document(self.0, &url)?;
        Wire::require(
            value["html_url"] == location
                && value["issue_url"]
                    == format!("https://api.github.com/repos/PerishLab/{origin}/issues/{issue}"),
            "semantic review comment coordinate differs",
        )?;
        Wire::require(
            value["id"].as_u64().map(|id| id.to_string()).as_deref() == Some(number),
            "semantic review native comment identity differs",
        )?;
        let comment = Wire::text(&value["node_id"])?;
        let body = value["body"]
            .as_str()
            .ok_or_else(|| Fault::refused("semantic review has no body"))?;
        let judgment = parse(body)?;
        let fields = serde_json::to_value(&judgment)
            .map_err(|_| Fault::refused("cannot bind semantic review product"))?;
        Wire::require(
            fields["product"] == product,
            "semantic review names another product",
        )?;
        let author = self.author(product, &value["user"])?;
        let current: Value = Wire::document(self.0, &url)?;
        Wire::require(
            current == value
                && Wire::repository(self.0, &format!("PerishLab/{origin}"))? == comments
                && Wire::repository(self.0, &format!("PerishLab/{product}"))? == repository,
            "semantic review or repository moved during observation",
        )?;
        Wire::require(
            self.author(product, &current["user"])? == author,
            "semantic review permission moved during observation",
        )?;
        Ok(Review {
            reference: Reference {
                url: location.into(),
                sha256: Wire::sha(body.as_bytes()),
                size: body.len() as u64,
            },
            judgment,
            author,
            comment: comment.into(),
            repository,
        })
    }
}
