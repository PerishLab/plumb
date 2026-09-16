use super::{Remote, Rule, failure, precondition};

impl Remote<'_> {
    pub fn publish(
        &self,
        bundle: &plumb::depot::v3::Bundle,
        source: &str,
        created: String,
    ) -> Result<String, String> {
        self.upload(bundle)?;
        self.promote(bundle, source, (created, None))
    }

    pub fn stage(&self, bundle: &plumb::depot::v3::Bundle, source: &str) -> Result<String, String> {
        self.upload(bundle)?;
        let held = &bundle.manifest;
        let generation = held.generation()?;
        let route = plumb::depot::v3::Route::new(&held.channel, held.kind, &held.version);
        let base = plumb::depot::v3::generation(route, &generation)?;
        super::readback::immutable(bundle, source, &base)?;
        let key = plumb::depot::v3::latest(route)?;
        let standing = self.pointer(&key)?;
        let expected = expectation(standing.as_ref())?;
        Ok(serde_json::json!({
            "marker": held.marker, "generation": generation, "expected": expected,
            "activated": false
        })
        .to_string())
    }

    pub fn promote(
        &self,
        bundle: &plumb::depot::v3::Bundle,
        source: &str,
        activation: (String, Option<&str>),
    ) -> Result<String, String> {
        let (created, expected) = activation;
        let held = &bundle.manifest;
        let generation = held.generation()?;
        let route = plumb::depot::v3::Route::new(&held.channel, held.kind, &held.version);
        let base = plumb::depot::v3::generation(route, &generation)?;
        super::readback::immutable(bundle, source, &base)?;
        let key = plumb::depot::v3::latest(route)?;
        let standing = self.pointer(&key)?;
        if let Some(expected) = expected {
            let current = expectation(standing.as_ref())?;
            let completed = standing.as_ref().is_some_and(|(_, pointer)| {
                pointer.projects(held) && pointer.generation == generation
            });
            if current != expected && !completed {
                return Err(format!(
                    "depot latest changed: expected {expected}, found {current}"
                ));
            }
        }
        let prior = standing
            .as_ref()
            .and_then(|(_, pointer)| pointer.projects(held).then(|| pointer.generation.clone()));
        let next = plumb::depot::v3::Pointer::new(
            held,
            plumb::depot::v3::Publication {
                source,
                prior,
                created,
            },
        )?;
        let pointer = self.project(&key, standing, &next)?;
        super::readback::public(&format!("{source}/{key}"), &pointer.encode()?)?;
        Ok(format!(
            "published {} depot generation {} {} and advanced latest",
            held.kind.directory(),
            held.version,
            generation
        ))
    }

    fn upload(&self, bundle: &plumb::depot::v3::Bundle) -> Result<(), String> {
        let held = &bundle.manifest;
        let route = plumb::depot::v3::Route::new(&held.channel, held.kind, &held.version);
        let base = plumb::depot::v3::generation(route, &held.generation()?)?;
        for (path, bytes) in &bundle.bodies {
            self.create(&format!("{base}/objects/{path}"), bytes)?;
        }
        self.create(
            &format!("{base}/{}", plumb::depot::v3::LEAF),
            &held.encode()?,
        )
    }

    fn pointer(&self, key: &str) -> Result<Option<(String, plumb::depot::v3::Pointer)>, String> {
        let Some(etag) = self.head(key)? else {
            return Ok(None);
        };
        let bytes = self
            .read(key)?
            .ok_or_else(|| format!("depot pointer vanished: {key}"))?;
        if self.head(key)?.as_deref() != Some(&etag) {
            return Err(format!("depot pointer changed while reading {key}"));
        }
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

fn expectation(standing: Option<&(String, plumb::depot::v3::Pointer)>) -> Result<String, String> {
    match standing {
        Some((_, pointer)) => Ok(plumb::depot::sha(&pointer.encode()?)),
        None => Ok("absent".into()),
    }
}
