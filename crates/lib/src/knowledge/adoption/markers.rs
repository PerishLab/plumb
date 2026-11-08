use super::super::{Inspector, wire::Wire};
use crate::semantics::version::Markers;
use crate::windtun::qualification::{Fault, Read};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Tag {
    pub marker: String,
    pub commit: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Inventory {
    pub repository: Value,
    pub markers: Vec<Tag>,
}

struct Reader<'a, R>(&'a R);

impl<R: Read> Inspector<R> {
    pub fn markers(&self, product: &str) -> Result<Inventory, Fault> {
        Wire::name(product)?;
        let name = format!("PerishLab/{product}");
        let repository = Wire::repository(&self.0, &name)?;
        let markers = Reader(&self.0).collect(&name)?;
        if Reader(&self.0).collect(&name)? != markers
            || Wire::repository(&self.0, &name)? != repository
        {
            return Err(Fault::unknown(
                "release tags or repository moved during inventory",
            ));
        }
        Ok(Inventory {
            repository,
            markers,
        })
    }
}

impl<R: Read> Reader<'_, R> {
    fn collect(&self, repository: &str) -> Result<Vec<Tag>, Fault> {
        let mut markers = Vec::new();
        let mut names = BTreeSet::new();
        let mut count = 0;
        for page in 1..=42 {
            let value: Value = Wire::document(
                self.0,
                &format!("https://api.github.com/repos/{repository}/tags?per_page=100&page={page}"),
            )?;
            let entries = value
                .as_array()
                .filter(|entries| entries.len() <= 100)
                .ok_or_else(|| Fault::refused("native tag page is absent or unbounded"))?;
            count += entries.len();
            if count > 4096 {
                return Err(Fault::unknown("native tag inventory exceeds its bound"));
            }
            for entry in entries {
                let name = Wire::text(&entry["name"])?;
                if !names.insert(name.to_string()) {
                    return Err(Fault::unknown("native tag pages repeat a marker"));
                }
                let commit = Self::commit(repository, entry)?;
                if name.starts_with('v') {
                    Markers::parse(name).map_err(Fault::refused)?;
                    markers.push(Tag {
                        marker: name.into(),
                        commit,
                    });
                }
            }
            if entries.len() < 100 {
                markers.sort_by(|a, b| a.marker.cmp(&b.marker));
                return Ok(markers);
            }
        }
        Err(Fault::unknown("native tag inventory is incomplete"))
    }

    fn commit(repository: &str, entry: &Value) -> Result<String, Fault> {
        let commit = Wire::text(&entry["commit"]["sha"])?;
        Wire::hex(commit, 40)?;
        Wire::require(
            entry["commit"]["url"]
                == format!("https://api.github.com/repos/{repository}/commits/{commit}"),
            "native tag points outside its repository commit namespace",
        )?;
        Ok(commit.into())
    }
}
