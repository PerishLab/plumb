use super::{Remote, Rule, failure, precondition};

impl Remote<'_> {
    pub fn publish(
        &self,
        bundle: &plumb::depot::v3::Bundle,
        source: &str,
        created: String,
    ) -> Result<String, String> {
        let held = &bundle.manifest;
        let generation = held.generation()?;
        let route = plumb::depot::v3::Route::new(&held.channel, held.kind, &held.version);
        let base = plumb::depot::v3::generation(route, &generation)?;
        for (path, bytes) in &bundle.bodies {
            self.create(&format!("{base}/objects/{path}"), bytes)?;
        }
        let body = held.encode()?;
        self.create(&format!("{base}/{}", plumb::depot::v3::LEAF), &body)?;
        let key = plumb::depot::v3::latest(route)?;
        let standing = self.pointer(&key)?;
        let next = plumb::depot::v3::Pointer::new(
            held,
            plumb::depot::v3::Publication {
                source,
                prior: standing.as_ref().map(|(_, held)| held.generation.clone()),
                created,
            },
        )?;
        let pointer = self.project(&key, standing, &next)?;
        super::readback::generation(super::readback::Generation {
            bundle,
            base: &base,
            key: &key,
            manifest: &body,
            pointer: &pointer,
        })?;
        Ok(format!(
            "published {} depot generation {} {} and advanced latest",
            held.kind.directory(),
            held.version,
            generation
        ))
    }

    fn pointer(&self, key: &str) -> Result<Option<(String, plumb::depot::v3::Pointer)>, String> {
        let Some(etag) = self.head(key)? else {
            return Ok(None);
        };
        let bytes = self
            .read(key)?
            .ok_or_else(|| format!("depot pointer vanished: {key}"))?;
        plumb::depot::v3::Pointer::parse(&bytes).map(|pointer| Some((etag, pointer)))
    }

    fn project(
        &self,
        key: &str,
        standing: Option<(String, plumb::depot::v3::Pointer)>,
        next: &plumb::depot::v3::Pointer,
    ) -> Result<plumb::depot::v3::Pointer, String> {
        let bytes = next.encode()?;
        let rule = match standing {
            None => Rule {
                cache: "public, max-age=60, must-revalidate",
                header: "--if-none-match",
                value: "*",
            },
            Some((_, ref current)) if !current.advance(next)? => return Ok(current.clone()),
            Some((ref etag, _)) => Rule {
                cache: "public, max-age=60, must-revalidate",
                header: "--if-match",
                value: etag,
            },
        };
        let output = self.put(key, &bytes, rule)?;
        if output.status.success() {
            return Ok(next.clone());
        }
        if !precondition(&output) {
            return Err(failure("move generation pointer", key, &output));
        }
        let current = self
            .read(key)?
            .ok_or_else(|| format!("generation pointer raced and {key} vanished"))?;
        let current = plumb::depot::v3::Pointer::parse(&current)?;
        if !current.advance(next)? {
            Ok(current)
        } else {
            Err(format!("generation pointer changed while advancing {key}"))
        }
    }
}
