use super::{Authority, Native, Record, Reference, policy, wire::Wire};
use crate::windtun::qualification::{Fault, Read};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Debug, Serialize)]
pub struct Origin {
    pub reference: Reference,
    pub record: Record,
    pub authority: Authority,
    pub producer: Value,
    pub job: Value,
    pub authentication: bool,
    pub publication: bool,
}

pub(super) struct Reader<'a, R>(pub &'a R);

impl<R: Read> Reader<'_, R> {
    pub fn observe(&self, reference: &Reference, product: &str) -> Result<Origin, Fault> {
        reference.validate(product)?;
        let authority = policy::Reader(self.0).current()?;
        let raw = reference.bytes(self.0, product)?;
        let record = Record::parse(&raw)?;
        Wire::require(
            record.product() == product,
            "semantic issuer names another product",
        )?;
        self.lineage(&record.native().commit, &authority.source.commit)?;
        let producer = self.run(record.native())?;
        let job = self.job(record.native(), &reference.sha256)?;
        self.run(record.native())?;
        Wire::require(
            self.job(record.native(), &reference.sha256)? == job
                && reference.bytes(self.0, product)? == raw
                && policy::Reader(self.0).current()? == authority,
            "semantic object, producer or authority moved during observation",
        )?;
        Ok(Origin {
            reference: reference.clone(),
            record,
            authority,
            producer,
            job,
            authentication: true,
            publication: false,
        })
    }

    pub(super) fn lineage(&self, commit: &str, head: &str) -> Result<(), Fault> {
        let compared: Value = Wire::document(
            self.0,
            &format!("{}/compare/{commit}...{head}", Wire::WHARF),
        )?;
        Wire::require(
            compared["base_commit"]["sha"] == commit
                && compared["merge_base_commit"]["sha"] == commit
                && matches!(compared["status"].as_str(), Some("ahead" | "identical")),
            "semantic issuer source is outside protected main history",
        )
    }

    pub(super) fn run(&self, native: &Native) -> Result<Value, Fault> {
        let value: Value = Wire::document(self.0, &Self::path(native))?;
        Wire::require(
            value["id"].as_u64() == Some(native.run)
                && value["run_attempt"].as_u64() == Some(native.attempt)
                && value["head_sha"] == native.commit,
            "semantic issuer attempt identity differs",
        )?;
        Wire::require(
            value["head_branch"] == "main"
                && value["event"] == "workflow_dispatch"
                && value["path"] == ".github/workflows/ship.yml",
            "semantic issuer is not its exact main ship workflow",
        )?;
        Wire::require(
            matches!(value["status"].as_str(), Some("in_progress" | "completed")),
            "semantic issuer attempt has not started",
        )?;
        for field in ["repository", "head_repository"] {
            Wire::require(
                value[field]["id"] == 1374358737_u64
                    && value[field]["node_id"] == "R_kgDOUesM0Q"
                    && value[field]["full_name"] == "PerishLab/wharf",
                "semantic issuer repository identity differs",
            )?;
        }
        Ok(value)
    }

    fn job(&self, native: &Native, digest: &str) -> Result<Value, Fault> {
        self.role(
            native,
            "[issue] version semantics",
            &format!("[issue] semantics {digest}"),
        )
    }

    pub(super) fn role(
        &self,
        native: &Native,
        role: &str,
        confirmation: &str,
    ) -> Result<Value, Fault> {
        let held: Value =
            Wire::document(self.0, &format!("{}/jobs?per_page=100", Self::path(native)))?;
        let jobs = held["jobs"]
            .as_array()
            .ok_or_else(|| Fault::refused("semantic issuer jobs are unavailable"))?;
        Wire::require(
            !jobs.is_empty()
                && jobs.len() <= 100
                && held["total_count"].as_u64() == Some(jobs.len() as u64),
            "semantic issuer job inventory is incomplete",
        )?;
        let mut ids = BTreeSet::new();
        for job in jobs {
            let id = job["id"].as_u64().unwrap_or(0);
            Wire::require(
                id > 0 && ids.insert(id),
                "semantic issuer job identity is absent or repeats",
            )?;
            Wire::require(
                job["run_id"].as_u64() == Some(native.run)
                    && job["run_attempt"].as_u64() == Some(native.attempt)
                    && job["head_sha"] == native.commit,
                "semantic issuer job attempt differs",
            )?;
            Wire::require(
                job["head_branch"] == "main",
                "semantic issuer job is outside main",
            )?;
        }
        let selected = jobs
            .iter()
            .filter(|job| job["name"] == role)
            .collect::<Vec<_>>();
        Wire::require(
            selected.len() == 1,
            "semantic issuer role is absent or ambiguous",
        )?;
        let job = selected[0];
        Wire::require(
            job["id"].as_u64() == Some(native.job)
                && job["status"] == "completed"
                && job["conclusion"] == "success",
            "semantic issuer job did not complete successfully",
        )?;
        let steps = job["steps"]
            .as_array()
            .ok_or_else(|| Fault::refused("semantic issuer steps are unavailable"))?;
        let confirmed = steps
            .iter()
            .filter(|step| step["name"] == confirmation)
            .collect::<Vec<_>>();
        Wire::require(
            confirmed.len() == 1
                && confirmed[0]["status"] == "completed"
                && confirmed[0]["conclusion"] == "success",
            "semantic issuer did not confirm the exact object bytes",
        )?;
        Ok(job.clone())
    }

    fn path(native: &Native) -> String {
        format!(
            "{}/actions/runs/{}/attempts/{}",
            Wire::WHARF,
            native.run,
            native.attempt
        )
    }
}
