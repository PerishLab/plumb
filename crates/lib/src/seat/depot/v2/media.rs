use super::{Kind, Manifest, Pointer, latest, snapshots};

pub struct Query<'a> {
    pub source: &'a str,
    pub product: &'a str,
    pub channel: &'a str,
    pub version: &'a str,
    pub derivative: Kind,
}

pub struct Generation {
    pub pointer: Pointer,
    pub manifest: Manifest,
    base: String,
}

impl Generation {
    pub fn latest(query: Query<'_>) -> Result<Option<Self>, String> {
        let source = query.source.trim_end_matches('/');
        let key = latest(query.product, query.derivative, query.channel)?;
        let Some(bytes) = pull(&format!("{source}/{key}"))? else {
            return Ok(None);
        };
        let text = String::from_utf8(bytes)
            .map_err(|error| format!("depot pointer is not UTF-8: {error}"))?;
        let pointer = Pointer::parse(&text)?;
        let standing = (
            pointer.source.trim_end_matches('/'),
            pointer.release.product.as_str(),
            pointer.release.channel.as_str(),
            pointer.release.version.as_str(),
            pointer.derivative,
        );
        let wanted = (
            source,
            query.product,
            query.channel,
            query.version,
            query.derivative,
        );
        if standing != wanted {
            return Ok(None);
        }
        Self::exact(source, pointer).map(Some)
    }

    pub fn exact(source: &str, pointer: Pointer) -> Result<Self, String> {
        let source = source.trim_end_matches('/');
        if pointer.source.trim_end_matches('/') != source {
            return Err("depot pointer names another source".into());
        }
        let route = snapshots(
            &pointer.release,
            pointer.derivative,
            &pointer.snapshot.timestamp,
        )?;
        let base = format!("{source}/{route}");
        let bytes = pull(&format!("{base}/{}", super::LEAF))?.ok_or_else(|| {
            format!(
                "depot generation {} has no manifest",
                pointer.snapshot.timestamp
            )
        })?;
        let text = String::from_utf8(bytes.clone())
            .map_err(|error| format!("depot manifest is not UTF-8: {error}"))?;
        let manifest = Manifest::parse(&text)?;
        pointer.bind(&manifest, &bytes)?;
        Ok(Self {
            pointer,
            manifest,
            base,
        })
    }

    pub fn read(&self, path: &str) -> Result<Vec<u8>, String> {
        let bytes = pull(&format!("{}/{path}", self.base))?
            .ok_or_else(|| format!("depot object {path} is absent"))?;
        self.manifest.verify(path, &bytes)?;
        Ok(bytes)
    }

    pub fn digest(&self) -> &str {
        &self.pointer.manifest.sha256
    }

    pub fn mark(&self) -> &str {
        &self.pointer.snapshot.timestamp
    }
}

fn pull(url: &str) -> Result<Option<Vec<u8>>, String> {
    let response = match ureq::get(url).call() {
        Ok(response) => response,
        Err(ureq::Error::StatusCode(404)) => return Ok(None),
        Err(error) => return Err(format!("cannot fetch {url}: {error}")),
    };
    let mut body = response.into_body();
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut body.as_reader(), &mut bytes)
        .map_err(|error| format!("cannot read {url}: {error}"))?;
    Ok(Some(bytes))
}
