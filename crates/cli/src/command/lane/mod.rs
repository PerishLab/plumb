mod forge;
pub(crate) mod source;

use crate::shape::lane::{Evidence, Projection};
use crate::shape::release::Spec;
use std::collections::BTreeMap;
use std::path::Path;

pub struct Seat<'a>(pub &'a Path);

struct Expected {
    path: String,
    rendered: String,
}

impl Seat<'_> {
    fn render(&self) -> Result<Vec<Expected>, String> {
        let spec = Spec::read(&self.0.join("plumb.toml"))?;
        let mut lanes = Vec::new();
        if !spec.surface().is_empty() {
            lanes.push(self.ship(&spec)?);
        }
        let refused = lanes
            .iter()
            .flat_map(|lane| forge::refusals(&lane.path, &lane.rendered))
            .collect::<Vec<_>>();
        if refused.is_empty() {
            Ok(lanes)
        } else {
            Err(refused.join("; "))
        }
    }

    fn ship(&self, spec: &Spec) -> Result<Expected, String> {
        let media = spec.surface();
        let carried = media.contains(&"binary");
        let projected = media.iter().any(|medium| *medium != "binary");
        let templates = if spec.product == "plumb" {
            source::Store::Factory
        } else {
            source::Store::Depot
        };
        let mut vars = BTreeMap::from([
            (
                "forge",
                crate::catalog::set::current().release.forge.clone(),
            ),
            ("plumb", manager("plumb")),
            ("after", if carried { ", seal" } else { "" }.to_string()),
        ]);
        let bootstrap = templates.filled("assets/ship/install.yml.in", &vars)?;
        let source = if spec.product == "plumb" {
            templates.text("assets/ship/source.yml.in")?
        } else {
            String::new()
        };
        let carry = format!("{}\n{source}", templates.matrixed(&bootstrap, &vars)?).replace(
            "if: runner.os",
            "if: matrix.control != 'reuse' && runner.os",
        );
        vars.insert("carry", carry);
        vars.insert("install", format!("{bootstrap}\n{source}"));
        let binary = if carried {
            templates.filled("assets/ship/binary.yml.in", &vars)?
        } else {
            String::new()
        };
        vars.insert(
            "capsule",
            if carried {
                templates.text("assets/ship/capsule.yml.in")?
            } else {
                String::new()
            },
        );
        let project = if projected {
            templates.filled("assets/ship/project.yml.in", &vars)?
        } else {
            String::new()
        };
        vars.insert("binary", binary);
        vars.insert("project", project);
        self.seat(
            "ship.yml",
            templates.filled("assets/ship/lane.yml.in", &vars)?,
        )
    }

    fn seat(&self, name: &str, rendered: String) -> Result<Expected, String> {
        let path = format!(".forgejo/workflows/{name}");
        Ok(Expected {
            path,
            rendered: format!("{}\n", rendered.trim_end()),
        })
    }

    pub fn write(&self, lanes: &[Projection]) -> Result<Vec<String>, String> {
        let mut written = Vec::new();
        for lane in lanes.iter().filter(|lane| lane.drifted()) {
            let path = self.0.join(&lane.path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
            }
            std::fs::write(&path, &lane.rendered)
                .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
            written.push(lane.path.clone());
        }
        Ok(written)
    }

    pub fn observe(&self, evidence: &mut Evidence) -> Result<(), String> {
        evidence.project(
            self.render()?
                .into_iter()
                .map(|lane| (lane.path, lane.rendered)),
        );
        Ok(())
    }

    pub fn project(&self) -> Result<Evidence, String> {
        let mut evidence = crate::shape::lane::read(self.0);
        self.observe(&mut evidence)?;
        Ok(evidence)
    }

    pub fn stale(&self) -> Vec<String> {
        let Ok(evidence) = self.project() else {
            return Vec::new();
        };
        evidence
            .projected()
            .iter()
            .filter(|lane| lane.drifted() && !lane.absent())
            .map(|lane| lane.path.clone())
            .collect()
    }
}

fn manager(tool: &str) -> String {
    format!("https://releases.{tool}.perish.uk")
}
