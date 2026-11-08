use super::super::{World, wire::Wire};
use crate::windtun::qualification::{Fault, Read, source::Source, world::plan};
use serde_json::{Value, json};

pub(super) struct Reader<'a, R>(pub &'a R);

impl<R: Read> Reader<'_, R> {
    pub fn observe(&self, world: &World) -> Result<Value, Fault> {
        let source = &world.source;
        let product = source.repository.strip_prefix("PerishLab/").unwrap();
        let selected = Source(self.0).commit(&source.repository, &source.marker)?;
        Wire::require(
            selected["source"]
                == serde_json::to_value(source)
                    .map_err(|_| Fault::refused("cannot encode adopted source"))?,
            "adopted source differs from its actual annotated marker",
        )?;
        let location = format!(
            "https://releases.{product}.perish.uk/v1/releases/stable/{}/distribution.json",
            source.marker
        );
        let record: Value = Wire::document(self.0, &location)?;
        plan::admit(&record)?;
        Wire::require(
            record["schema"] == 1 && record["state"] == "complete",
            "adoption requires complete formal distribution",
        )?;
        if record.get("plan").is_none() {
            return Err(Fault::unknown(
                "adopted publication has no original plan reference",
            ));
        }
        let point = serde_json::to_value(source)
            .map_err(|_| Fault::refused("cannot encode adopted source"))?;
        for field in ["repository", "marker", "commit", "tree"] {
            Wire::require(
                record[field] == point[field],
                "adopted distribution source differs",
            )?;
        }
        let mut combination = serde_json::to_value(world)
            .map_err(|_| Fault::refused("cannot encode adopted original world"))?;
        combination.as_object_mut().unwrap().remove("guard");
        let resolutions = record["resolution"]
            .as_object()
            .filter(|held| !held.is_empty())
            .ok_or_else(|| Fault::unknown("adopted distribution has no recorded combinations"))?;
        Wire::require(
            resolutions.values().all(|value| *value == combination),
            "adopted world differs from recorded publication combinations",
        )?;
        Ok(json!({"distribution":record,"tag":selected["tag"]}))
    }
}
