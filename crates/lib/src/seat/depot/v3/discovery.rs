use std::collections::BTreeMap;
use std::path::Path;

use super::remote::{pull, read};
use super::value::Value;
use super::{Bundle, Cache, Generation, Kind, LEAF, Manifest, Pointer, Route};

#[derive(Clone, Copy, Debug)]
pub struct Channel<'a> {
    pub source: &'a str,
    pub product: &'a str,
    pub channel: &'a str,
    pub kind: Kind,
}

#[derive(Debug)]
pub struct Discovered {
    pub pointer: Pointer,
    pub bundle: Bundle,
}

impl Channel<'_> {
    pub fn pointer(&self) -> Result<Pointer, String> {
        Value(self.source).source()?;
        Value(self.product).component("depot product")?;
        let source = self.source.trim_end_matches('/');
        let key = super::channel(self.channel, self.kind)?;
        let bytes = pull(&format!("{source}/{key}"))?
            .ok_or_else(|| format!("depot channel pointer is absent: {source}/{key}"))?;
        let pointer = Pointer::parse(&bytes)?;
        self.matchup(&pointer, source)?;
        self.cross(&pointer, source)?;
        Ok(pointer)
    }

    pub fn resolve(&self) -> Result<Generation, String> {
        let pointer = self.pointer()?;
        Generation::exact(self.source, pointer)
    }

    pub fn fetch(&self) -> Result<Discovered, String> {
        let pointer = self.pointer()?;
        let (bundle, _) = download(self.source, &pointer)?;
        Ok(Discovered { pointer, bundle })
    }

    pub fn cached(&self, home: &Path) -> Result<Discovered, String> {
        let pointer = self.pointer()?;
        let cache = Cache::new(home);
        if cache.has(&pointer.generation)? {
            let bundle = cache.read(&pointer)?;
            return Ok(Discovered { pointer, bundle });
        }
        let (bundle, raw) = download(self.source, &pointer)?;
        cache.store(&pointer, &raw, &bundle)?;
        Ok(Discovered { pointer, bundle })
    }

    fn matchup(&self, pointer: &Pointer, source: &str) -> Result<(), String> {
        if pointer.product != self.product
            || pointer.channel != self.channel
            || pointer.kind != self.kind
        {
            return Err(format!(
                "depot channel pointer names {}/{}/{} instead of {}/{}/{}",
                pointer.product,
                pointer.channel,
                pointer.kind.label(),
                self.product,
                self.channel,
                self.kind.label()
            ));
        }
        let route = Route::new(&pointer.channel, pointer.kind, &pointer.version);
        let expected = super::manifest(source, route, &pointer.generation)?;
        if pointer.manifest.url != expected {
            return Err("depot channel pointer names another source".into());
        }
        Ok(())
    }

    fn cross(&self, pointer: &Pointer, source: &str) -> Result<(), String> {
        let route = Route::new(&pointer.channel, pointer.kind, &pointer.version);
        let key = super::latest(route)?;
        let bytes = pull(&format!("{source}/{key}"))?
            .ok_or_else(|| format!("depot standing pointer is absent: {source}/{key}"))?;
        let standing = Pointer::parse(&bytes)?;
        let expected = super::manifest(source, route, &standing.generation)?;
        if standing.manifest.url != expected {
            return Err("depot standing pointer names another source".into());
        }
        if standing.identity() != pointer.identity() {
            return Err("depot channel pointer disagrees with version standing identity".into());
        }
        if standing.generation == pointer.generation && standing.manifest != pointer.manifest {
            return Err("depot channel pointer disagrees with version standing manifest".into());
        }
        Ok(())
    }
}

fn download(source: &str, pointer: &Pointer) -> Result<(Bundle, Vec<u8>), String> {
    let source = source.trim_end_matches('/');
    let route = Route::new(&pointer.channel, pointer.kind, &pointer.version);
    let expected = super::manifest(source, route, &pointer.generation)?;
    if pointer.manifest.url != expected {
        return Err("depot pointer names another source".into());
    }
    let raw = pull(&pointer.manifest.url)?
        .ok_or_else(|| format!("depot generation {} has no manifest", pointer.generation))?;
    let manifest = Manifest::parse(&raw)?;
    pointer.bind(&manifest, &raw)?;
    let base = pointer
        .manifest
        .url
        .strip_suffix(LEAF)
        .expect("a valid depot manifest URL ends in its leaf");
    let mut bodies = BTreeMap::new();
    for object in &manifest.objects {
        let bytes = read(base, &manifest, &object.path)?;
        bodies.insert(object.path.clone(), bytes);
    }
    Ok((Bundle { manifest, bodies }, raw))
}
