use super::{
    Inspector, Read, Result,
    receipt::Receipt,
    source::Source,
    wire::{WHARF, Wire},
    world::World,
};
use serde_json::{Value, json};
use std::collections::BTreeSet;

impl<R: Read> Inspector<R> {
    pub(super) fn verify(&self, repository: &str, marker: &str) -> Result<Value> {
        let source = Source(&self.reader);
        let subject = source.subject(repository, marker)?;
        let policy = source.policy()?;
        let receipt = Receipt::load(&self.reader, &subject, &policy["requirements"])?;
        self.native(&receipt)?;
        let world = World::new(&self.reader, &subject);
        let pins = world.verify(&receipt.passed)?;
        source.unchanged(&subject)?;
        Wire::require(
            world.pins()? == pins,
            "current product, runner, scene or image moved",
        )?;
        let standing = source.policy()?;
        Wire::require(
            standing["requirements"] == policy["requirements"],
            "current verification requirements moved",
        )?;
        receipt.unchanged(&self.reader)?;
        Ok(
            json!({"requirements":policy, "final_requirements":standing, "intent":receipt.intent, "credential":receipt.result["credential"], "observer":receipt.index["observer"], "source":subject.source, "tag":subject.tag, "world":pins}),
        )
    }

    fn native(&self, receipt: &Receipt) -> Result<()> {
        let observer = &receipt.index["observer"];
        let run = Wire::text(&observer["run"])?;
        let attempt = Wire::text(&observer["attempt"])?;
        let root = format!("{WHARF}/actions/runs/{run}/attempts/{attempt}");
        let metadata = Wire::document(&self.reader, &root, false)?;
        Wire::require(
            Wire::number(&metadata["id"])? == run.parse().unwrap_or(0)
                && Wire::number(&metadata["run_attempt"])? == attempt.parse().unwrap_or(0)
                && metadata["head_sha"] == observer["wharf"],
            "qualification native attempt differs",
        )?;
        Wire::matches(
            &metadata,
            &json!({"head_branch":"main", "path":".github/workflows/ship.yml", "event":"workflow_dispatch", "status":"completed"}),
            "qualification is not completed trusted main ship code",
        )?;
        for field in ["repository", "head_repository"] {
            Wire::require(
                metadata[field]["id"] == 1374358737_u64
                    && metadata[field]["full_name"] == "PerishLab/wharf",
                "qualification producer repository differs",
            )?;
        }
        let held = Wire::document(&self.reader, &format!("{root}/jobs?per_page=100"), false)?;
        let jobs = held["jobs"]
            .as_array()
            .ok_or_else(|| super::Fault::refused("missing native job inventory"))?;
        Wire::require(
            !jobs.is_empty()
                && jobs.len() <= 100
                && Wire::number(&held["total_count"])? == jobs.len() as u64,
            "native job inventory is incomplete",
        )?;
        let mut identities = BTreeSet::new();
        for job in jobs {
            Wire::require(
                identities.insert(Wire::number(&job["id"])?)
                    && job["id"] != 0
                    && Wire::number(&job["run_id"])? == run.parse().unwrap_or(0)
                    && Wire::number(&job["run_attempt"])? == attempt.parse().unwrap_or(0)
                    && job["head_sha"] == observer["wharf"]
                    && job["head_branch"] == "main",
                "native job identity differs or repeats",
            )?;
        }
        let prepare = Self::job(jobs, "[windtun] prepare attempt")?;
        Wire::require(
            prepare["started_at"] == receipt.index["started"],
            "native attempt start differs",
        )?;
        Self::steps(
            prepare,
            &["[windtun] admit publisher", "[windtun] prepare attempt"],
        )?;
        Self::steps(
            Self::job(jobs, "[windtun] published artifacts")?,
            &[
                "[windtun] load attempt",
                "[windtun] published artifacts",
                "[windtun] judge observation",
                "[windtun] record attempt",
            ],
        )?;
        Self::steps(
            Self::job(jobs, "[windtun] issue qualification")?,
            &["[windtun] issue qualification"],
        )
    }

    fn job<'a>(jobs: &'a [Value], name: &str) -> Result<&'a Value> {
        let matched: Vec<_> = jobs.iter().filter(|job| job["name"] == name).collect();
        Wire::require(
            matched.len() == 1,
            "native qualification role is absent or ambiguous",
        )?;
        Ok(matched[0])
    }

    fn steps(job: &Value, names: &[&str]) -> Result<()> {
        Wire::require(
            job["status"] == "completed" && job["conclusion"] == "success",
            "native qualification role did not succeed",
        )?;
        let steps = job["steps"]
            .as_array()
            .ok_or_else(|| super::Fault::refused("native steps are missing"))?;
        for name in names {
            let selected: Vec<_> = steps.iter().filter(|step| step["name"] == *name).collect();
            Wire::require(
                selected.len() == 1
                    && selected[0]["status"] == "completed"
                    && selected[0]["conclusion"] == "success",
                "native qualification step did not succeed",
            )?;
        }
        Ok(())
    }
}
