use super::SCHEMA;
use super::context::Context;
use super::inputs::Inputs;
use super::inputs::Stamp;
use super::record::Hint;
use super::target::Target;
use crate::command::guard::precommit::cargo::Lease;
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) struct Session<'a> {
    root: &'a Path,
    lease: &'a Lease,
    world: &'a str,
    context: &'a Context,
}

impl<'a> Session<'a> {
    pub fn begin(
        root: &'a Path,
        lease: &'a Lease,
        world: &'a str,
        context: &'a Context,
    ) -> Result<Option<Self>, String> {
        let session = Self {
            root,
            lease,
            world,
            context,
        };
        let bucket = &lease.root;
        let hint = Hint::read(bucket)?;
        let inputs = Inputs(root).read()?;
        if !Target(bucket).supported()? || inputs.is_none() {
            if hint.is_some() {
                session.pending()?.write(bucket)?;
            }
            return Ok(None);
        }
        let inputs = inputs.expect("supported inputs were read");
        let actual = Target(bucket).seal()?;
        let now = Stamp::now()?;
        let valid = hint
            .as_ref()
            .filter(|hint| {
                hint.source == root && hint.world == world && hint.image == context.image
            })
            .filter(|hint| hint.valid(&actual, now));
        match valid {
            Some(hint) => Inputs(root).restore(&inputs, &hint.files, hint.stamp.fresh()?)?,
            None => {
                let cause = cause(hint.as_ref(), root, world, context);
                crate::diagnostic::event(
                    "material.reset",
                    || serde_json::json!({"reason":"snapshot-missing-pending-or-mismatched","cause":cause,"files":actual.files}),
                );
                if actual.files != 0 {
                    eprintln!(
                        "guard material: snapshot missing, pending or mismatched; recompiling native Cargo material"
                    );
                }
                crate::diagnostic::measure("material.reset", || Target(bucket).reset())?;
            }
        }
        session.pending()?.write(bucket)?;
        Ok(Some(session))
    }

    fn pending(&self) -> Result<Hint, String> {
        Ok(Hint {
            schema: SCHEMA.into(),
            ready: false,
            source: self.root.to_path_buf(),
            world: self.world.into(),
            image: self.context.image.clone(),
            stamp: Stamp::now()?,
            target: None,
            files: BTreeMap::new(),
        })
    }

    pub fn seal(&self) -> Result<(), String> {
        let Some(files) = Inputs(self.root).read()? else {
            return Ok(());
        };
        let target = Target(&self.lease.root).seal()?;
        let stamp = Stamp::now()?;
        if target.latest > stamp || files.values().any(|input| input.stamp > stamp) {
            return Err(
                "Guard compilation material has future timestamps; snapshot remains pending".into(),
            );
        }
        let hint = Hint {
            schema: SCHEMA.into(),
            ready: true,
            source: self.root.to_path_buf(),
            world: self.world.into(),
            image: self.context.image.clone(),
            stamp,
            target: Some(target),
            files,
        };
        hint.write(&self.lease.root)
    }
}

pub(super) fn cause(
    hint: Option<&Hint>,
    root: &Path,
    world: &str,
    context: &Context,
) -> &'static str {
    match hint {
        None => "missing",
        Some(hint) if !hint.ready => "pending",
        Some(hint) if hint.source != root => "source",
        Some(hint) if hint.world != world => "world",
        Some(hint) if hint.image != context.image => "image",
        Some(_) => "seal",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn causes() {
        let root = Path::new("/seat");
        let context = Context {
            image: "image".into(),
        };
        let held = |ready: bool, source: &str, world: &str, image: &str| Hint {
            schema: SCHEMA.into(),
            ready,
            source: source.into(),
            world: world.into(),
            image: image.into(),
            stamp: Stamp::now().unwrap(),
            target: None,
            files: BTreeMap::new(),
        };
        assert_eq!(cause(None, root, "world", &context), "missing");
        let hint = held(false, "/seat", "world", "image");
        assert_eq!(cause(Some(&hint), root, "world", &context), "pending");
        let hint = held(true, "/other", "world", "image");
        assert_eq!(cause(Some(&hint), root, "world", &context), "source");
        let hint = held(true, "/seat", "old", "image");
        assert_eq!(cause(Some(&hint), root, "world", &context), "world");
        let hint = held(true, "/seat", "world", "old");
        assert_eq!(cause(Some(&hint), root, "world", &context), "image");
        let hint = held(true, "/seat", "world", "image");
        assert_eq!(cause(Some(&hint), root, "world", &context), "seal");
    }
}
