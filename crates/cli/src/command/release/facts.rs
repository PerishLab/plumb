use super::identity::deed::Purpose;
use plumb::semantics::admission::{Inspector, Record, World};
use plumb::windtun::qualification::{Fault, Remote, State};
use serde_json::{Value, json};
use std::io::Read;
use std::path::Path;

pub(super) struct Facts;

impl Facts {
    const LIMIT: usize = 4 * 1024 * 1024;

    pub fn policy() -> i32 {
        Self::print(
            Inspector::new(Remote::default())
                .current()
                .and_then(|held| Self::value(&held)),
        )
    }

    pub fn review(product: &str, reference: &str, purpose: Purpose) -> i32 {
        let inspector = Inspector::new(Remote::default());
        let result = match purpose {
            Purpose::Boundary => inspector
                .review(reference, product)
                .and_then(|held| Self::value(&held)),
            Purpose::Adoption => inspector
                .adoption(reference, product)
                .and_then(|held| Self::value(&held)),
        };
        Self::print(result)
    }

    pub fn inspect(path: &Path) -> i32 {
        Self::print(Self::read(path).and_then(|raw| {
            let value: Value = serde_json::from_slice(&raw)
                .map_err(|_| Fault::refused("invalid supplied JSON"))?;
            let held = if value["schema"] == World::SCHEMA {
                Self::value(&World::parse(&raw)?)?
            } else {
                Self::value(&Record::parse(&raw)?)?
            };
            Ok(json!({"schema":"plumb.release.inspection/v1","data":held,
                "authentication":false,"publication":false}))
        }))
    }

    fn read(path: &Path) -> Result<Vec<u8>, Fault> {
        let metadata = std::fs::symlink_metadata(path)
            .map_err(|_| Fault::refused("cannot inspect supplied input"))?;
        if !metadata.is_file() || metadata.len() > Self::LIMIT as u64 {
            return Err(Fault::refused(
                "input requires a regular JSON file within 4 MiB",
            ));
        }
        let file =
            std::fs::File::open(path).map_err(|_| Fault::refused("cannot open supplied input"))?;
        let mut raw = Vec::new();
        file.take((Self::LIMIT + 1) as u64)
            .read_to_end(&mut raw)
            .map_err(|_| Fault::refused("cannot read supplied input"))?;
        if raw.len() > Self::LIMIT {
            return Err(Fault::refused("supplied input exceeds 4 MiB"));
        }
        Ok(raw)
    }

    fn value(held: &impl serde::Serialize) -> Result<Value, Fault> {
        serde_json::to_value(held).map_err(|_| Fault::refused("cannot encode semantic facts"))
    }

    fn print(result: Result<Value, Fault>) -> i32 {
        let (held, code) = match result {
            Ok(value) => (
                json!({"state":"observed","facts":value,"publication":false}),
                0,
            ),
            Err(fault) => {
                let code = if fault.state == State::Unknown { 3 } else { 1 };
                (
                    json!({"state":fault.state,"reason":fault.reason,"publication":false}),
                    code,
                )
            }
        };
        match serde_json::to_string_pretty(&held) {
            Ok(body) => {
                println!("{body}");
                code
            }
            Err(error) => {
                eprintln!("plumb release: cannot encode facts: {error}");
                1
            }
        }
    }
}
