mod bundle;
mod scene;
mod source;
mod wire;

use super::{Snapshot, Source};
use crate::seat::depot::v3::Pointer;
use crate::windtun::qualification::{Fault, Read, State};
pub use scene::Scene;
use serde::Serialize;
use serde_json::Value;
use source::Subject;

#[derive(Debug, Serialize)]
pub struct Observation {
    pub source: Source,
    pub repository: Value,
    pub files: Value,
    pub scenes: Option<Scene>,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub schema: &'static str,
    pub state: &'static str,
    pub observation: Option<Observation>,
    pub snapshot: Option<Snapshot>,
    pub adoption: bool,
    pub publication: bool,
    pub reason: Option<String>,
}

impl Report {
    fn pending() -> Self {
        Self {
            schema: "plumb.release.collection/v1",
            state: "unknown",
            observation: None,
            snapshot: None,
            adoption: false,
            publication: false,
            reason: None,
        }
    }

    fn settle(&mut self, result: Result<(), Fault>) {
        if let Err(fault) = result {
            self.state = if fault.state == State::Refused {
                "refused"
            } else {
                "unknown"
            };
            self.reason = Some(fault.reason);
        }
    }
}

pub struct Collector<R>(R);

impl<R: Read> Collector<R> {
    pub fn new(reader: R) -> Self {
        Self(reader)
    }

    pub fn inspect(&self, repository: &str, commit: &str) -> Report {
        let mut held = Report::pending();
        let result = self.collect(repository, commit, &mut held);
        held.settle(result);
        held
    }

    pub fn historical(&self, repository: &str, commit: &str, pointer: &Pointer) -> Report {
        let mut held = Report::pending();
        let result = self.past((repository, commit), pointer, &mut held);
        held.settle(result);
        held
    }

    fn past(
        &self,
        selected: (&str, &str),
        pointer: &Pointer,
        report: &mut Report,
    ) -> Result<(), Fault> {
        let source = source::Reader(&self.0);
        let subject = source.inspect(selected.0, selected.1)?;
        report.observation = Some(Observation {
            source: subject.source.clone(),
            repository: subject.repository.clone(),
            files: subject.files.clone(),
            scenes: None,
        });
        let declaration = subject
            .declaration
            .as_ref()
            .ok_or_else(|| Fault::unknown("historical source has no Scene declaration"))?;
        let scene = scene::Reader(&self.0).historical(&declaration.scenes, pointer)?;
        report.observation.as_mut().unwrap().scenes = Some(scene);
        source.unchanged(&subject)?;
        let scene = report
            .observation
            .as_ref()
            .unwrap()
            .scenes
            .as_ref()
            .unwrap();
        report.snapshot = Some(Self::snapshot(&subject, scene)?);
        report.state = "observed";
        report.reason = Some("Historical immutable materials verified; no current-channel, adoption or publication claim".into());
        Ok(())
    }

    fn collect(&self, repository: &str, commit: &str, report: &mut Report) -> Result<(), Fault> {
        let source = source::Reader(&self.0);
        let subject = source.inspect(repository, commit)?;
        report.observation = Some(Observation {
            source: subject.source.clone(),
            repository: subject.repository.clone(),
            files: subject.files.clone(),
            scenes: None,
        });
        let declaration = subject.declaration.as_ref().ok_or_else(|| {
            Fault::unknown("source has no Scene declaration; boundary remains unknown")
        })?;
        let scene = scene::Reader(&self.0).inspect(&declaration.scenes)?;
        report.observation.as_mut().unwrap().scenes = Some(scene);
        let scene = report
            .observation
            .as_ref()
            .unwrap()
            .scenes
            .as_ref()
            .unwrap();
        source.unchanged(&subject)?;
        scene::Reader(&self.0).unchanged(&declaration.scenes, scene)?;
        report.snapshot = Some(Self::snapshot(&subject, scene)?);
        report.state = "observed";
        Ok(())
    }

    fn snapshot(subject: &Subject, scene: &scene::Scene) -> Result<Snapshot, Fault> {
        let mapping = scene.subjects.as_ref().ok_or_else(|| {
            Fault::unknown(
                "formal Scene has no subject metadata; historical absence is not a baseline",
            )
        })?;
        let product = subject
            .source
            .repository
            .strip_prefix("PerishLab/")
            .unwrap();
        let coverage = mapping.coverage(product).map_err(Fault::refused)?;
        if coverage.is_empty() {
            return Err(Fault::unknown(
                "Scene declares no normative coverage for this product",
            ));
        }
        let mut materials = scene.materials.clone();
        let mut files = subject.files.clone();
        files.sort_all_objects();
        let raw = serde_json::to_vec(&files)
            .map_err(|_| Fault::refused("cannot encode source projection"))?;
        materials.push(
            super::Material::new(super::Kind::Schema, "source.json".into(), &raw, false)
                .map_err(Fault::refused)?,
        );
        for case in &mapping.cases {
            if !coverage.iter().any(|item| item.case == case.id) {
                continue;
            }
            let raw = serde_json::to_vec(case)
                .map_err(|_| Fault::refused("cannot encode case projection"))?;
            materials.push(
                super::Material::new(super::Kind::Case, case.id.clone(), &raw, false)
                    .map_err(Fault::refused)?,
            );
        }
        let snapshot = Snapshot {
            product: product.into(),
            source: subject.source.clone(),
            materials,
            coverage,
        };
        snapshot.validate().map_err(Fault::refused)?;
        Ok(snapshot)
    }
}
