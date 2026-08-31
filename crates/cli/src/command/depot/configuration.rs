use super::store;
use crate::shape::depot as record;
use plumb::rig::Rig;
use plumb::snapshot::Snapshot;
use std::path::Path;
use std::process::Output;

pub struct Tree<'a>(pub &'a Path);

impl Tree<'_> {
    pub fn publish(&self, raw: &str, dry: bool) -> Result<String, String> {
        let mut rig = Rig::resolve(None).map_err(|error| error.to_string())?;
        let spec = crate::shape::release::Spec::read(&self.0.join("plumb.toml"))?;
        let marker = crate::command::release::ReleaseMarker::at(
            self.0,
            &spec.product,
            &spec.authority,
            raw,
        )?;
        let proof = marker.digest()?;
        let commit = self.commit()?;
        if marker.commit != commit {
            return Err(format!(
                "release marker {} seals {}, not HEAD at {commit}",
                marker.marker, marker.commit
            ));
        }
        let depot = spec.derivative(plumb::depot::v2::Kind::Configuration)?;
        let product = crate::command::release::Product::new(&spec);
        let release = product.depot();
        let binding = release.binding(&marker.marker, true)?;
        let snapshot = Snapshot::read(self.0).map_err(|error| error.to_string())?;
        self.clean()?;
        let plan = record::Batch::configuration(
            &snapshot,
            record::Draft {
                source: depot.source.clone(),
                release: binding.release.clone(),
                timestamp: super::super::clock::mark()?,
                commit,
            },
        )?;
        crate::command::release::validate_depot(&spec, &binding, &plan)?;
        let held = (|| {
            if dry {
                plan.manifest.encode()
            } else {
                rig.depot.authority.load()?;
                store::Remote::new(&rig.depot.authority)?.derive(&plan, false)
            }
        })();
        let after = crate::command::release::ReleaseMarker::at(
            self.0,
            &spec.product,
            &spec.authority,
            &marker.marker,
        )?;
        if after.digest()? != proof {
            return Err(format!(
                "release marker {} drifted while depot was deriving configuration",
                marker.marker
            ));
        }
        held
    }

    pub fn advance(&self, raw: &str) -> Result<String, String> {
        let mut rig = Rig::resolve(None).map_err(|error| error.to_string())?;
        let spec = crate::shape::release::Spec::read(&self.0.join("plumb.toml"))?;
        let marker = crate::command::release::ReleaseMarker::at(
            self.0,
            &spec.product,
            &spec.authority,
            raw,
        )?;
        let proof = marker.digest()?;
        let commit = self.commit()?;
        if marker.commit != commit {
            return Err(format!(
                "release marker {} seals {}, not HEAD at {commit}",
                marker.marker, marker.commit
            ));
        }
        let depot = spec.derivative(plumb::depot::v2::Kind::Configuration)?;
        let pointer = super::seat::exact(super::seat::Query {
            source: &depot.source,
            product: &spec.product,
            derivative: plumb::depot::v2::Kind::Configuration,
            channel: &marker.channel,
            version: &marker.marker,
        })?;
        let standing = (
            pointer.release.product.as_str(),
            pointer.release.channel.as_str(),
            pointer.release.version.as_str(),
            pointer.release.commit.as_str(),
        );
        let wanted = (
            marker.product.as_str(),
            marker.channel.as_str(),
            marker.marker.as_str(),
            marker.commit.as_str(),
        );
        if standing != wanted {
            return Err(format!(
                "configuration depot pointer does not bind release marker {}",
                marker.marker
            ));
        }
        rig.depot.authority.load()?;
        let held = store::Remote::new(&rig.depot.authority)?.promote(&pointer)?;
        let after = crate::command::release::ReleaseMarker::at(
            self.0,
            &spec.product,
            &spec.authority,
            &marker.marker,
        )?;
        if after.digest()? != proof {
            return Err(format!(
                "release marker {} drifted while depot advanced configuration",
                marker.marker
            ));
        }
        Ok(held)
    }

    fn clean(&self) -> Result<(), String> {
        let mut args = vec!["status", "--porcelain", "--"];
        let roots = record::configuration(self.0)?;
        for (root, _) in &roots {
            args.push(root);
        }
        let output = self.git(&args)?;
        if !output.status.success() {
            return Err(format!(
                "cannot read the working tree: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        let dirty = String::from_utf8_lossy(&output.stdout);
        if dirty.trim().is_empty() {
            Ok(())
        } else {
            Err(format!(
                "depot roots carry uncommitted change:\n{}",
                dirty.trim()
            ))
        }
    }

    pub fn commit(&self) -> Result<String, String> {
        let output = self.git(&["rev-parse", "HEAD"])?;
        if !output.status.success() {
            return Err("cannot read HEAD".to_string());
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn git(&self, args: &[&str]) -> Result<Output, String> {
        std::process::Command::new("git")
            .current_dir(self.0)
            .args(args)
            .output()
            .map_err(|error| format!("cannot run git: {error}"))
    }
}
