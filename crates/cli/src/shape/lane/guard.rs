use crate::shape::workflow;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub struct Proof {
    pub key: &'static str,
    pub line: String,
}

pub struct Seat(BTreeSet<String>);

impl Seat {
    pub fn read(root: &Path) -> Self {
        Self(
            workflow::read(root)
                .keys
                .iter()
                .map(workflow::Key::name)
                .collect(),
        )
    }

    pub fn guards(&self, key: &str) -> bool {
        self.0.contains(&name(key))
    }

    pub fn any(&self, listed: &[Proof]) -> bool {
        listed.iter().any(|proof| self.guards(proof.key))
    }

    pub fn when(&self, key: &str) -> String {
        if !self.guards(key) {
            return String::new();
        }
        format!(
            "        if: steps.workflow.outputs.{} != 'false'\n",
            output(key)
        )
    }

    pub fn steps(&self, listed: Vec<Proof>, proof: &str) -> Result<String, String> {
        let mut held: Vec<(&'static str, Vec<String>)> = Vec::new();
        for entry in listed {
            match held.last_mut() {
                Some((key, lines)) if *key == entry.key => lines.push(entry.line),
                _ => held.push((entry.key, vec![entry.line])),
            }
        }
        let mut blocks = Vec::new();
        for (key, mut lines) in held {
            if self.guards(key) {
                lines.push(format!("plumb workflow lock {} || true", name(key)));
            }
            blocks.push(self.block(key, &lines, proof)?);
        }
        Ok(blocks.join("\n"))
    }

    fn block(&self, key: &str, lines: &[String], proof: &str) -> Result<String, String> {
        let body = lines
            .iter()
            .map(|line| format!("          {line}"))
            .collect::<Vec<_>>()
            .join("\n");
        let vars = BTreeMap::from([
            ("key", key.to_string()),
            ("when", self.when(key)),
            ("lines", body),
        ]);
        plumb::fill::actions(proof, &vars).map_err(|error| error.to_string())
    }
}

pub const LOCK: &str = "    env:\n      PLUMB_WORKFLOW_SEAT: ${{ github.repository }}\n      PLUMB_LOCK_ACCESS: ${{ secrets.workflow_lock_s3_access_key }}\n      PLUMB_LOCK_SECRET: ${{ secrets.workflow_lock_s3_secret_key }}\n      PLUMB_LOCK_BUCKET: ${{ secrets.workflow_lock_s3_bucket }}\n      PLUMB_LOCK_ENDPOINT: ${{ secrets.workflow_lock_s3_endpoint }}\n";

fn name(key: &str) -> String {
    format!("guard/{key}")
}

fn output(key: &str) -> String {
    key.chars()
        .map(|held| {
            if held.is_ascii_alphanumeric() {
                held
            } else {
                '_'
            }
        })
        .collect()
}
