use plumb::depot::v3::Kind;
use plumb::forgejo::git;
use std::path::Path;

use super::repository::product::{Catalog, Definition, Document, Legacy};

const DOMAIN: &str = "git.perish.top";
const FACTORY: &str = "schema = \"plumb.products/v1\"\n";

pub struct Target {
    pub product: String,
    pub authority: String,
    derivatives: Vec<Kind>,
    pub profile: Option<Profile>,
}

pub struct Profile {
    pub configuration: String,
    pub digest: String,
    pub manifest: String,
    pub ectropy: String,
}

pub fn governance(root: &Path) -> Result<Option<Target>, String> {
    match git::remote(root, "") {
        Ok(remote) if remote.host == DOMAIN => {
            let rig = plumb::rig::Rig::resolve(None).map_err(|error| error.to_string())?;
            resolve(root, &rig.rules.source).map(Some)
        }
        _ => Ok(None),
    }
}

struct Root<'a>(&'a Path);

pub fn resolve(root: &Path, _: &str) -> Result<Target, String> {
    let seat = super::super::command::depot::held();
    configured(root, &seat)
}

pub fn at(repository: &Path, _: &str, seat: &plumb::depot::Rules) -> Result<Target, String> {
    configured(repository, seat)
}

trait Configuration {
    fn read(&self, path: &str, factory: &'static str) -> Result<String, String>;
    fn mark(&self) -> Option<String>;
}

impl Configuration for super::super::command::depot::Held {
    fn read(&self, path: &str, factory: &'static str) -> Result<String, String> {
        super::super::command::depot::Held::read(self, path, factory)
    }

    fn mark(&self) -> Option<String> {
        self.mark()
    }
}

impl Configuration for plumb::depot::Rules {
    fn read(&self, path: &str, _: &'static str) -> Result<String, String> {
        plumb::depot::Rules::read(self, path)
    }

    fn mark(&self) -> Option<String> {
        Some(self.mark().to_string())
    }
}

fn configured<C: Configuration>(repository: &Path, seat: &C) -> Result<Target, String> {
    let remote = git::remote(repository, "")?;
    if remote.host != DOMAIN {
        return Root(repository).manifested();
    }
    let identity = format!("{}/{}/{}", remote.host, remote.owner, remote.repo);
    let raw = seat.read("rules/products.toml", FACTORY)?;
    let schema = raw
        .parse::<toml::Table>()
        .ok()
        .and_then(|held| held.get("schema")?.as_str().map(str::to_string))
        .ok_or_else(|| "depot rules/products.toml names no schema".to_string())?;
    match schema.as_str() {
        "plumb.products/v1" => inline(&raw, &identity),
        "plumb.products/v2" => profiled(&raw, &identity, seat),
        _ => Err(format!("unknown product catalog schema {schema}")),
    }
}

pub fn guard(root: &Path, source: &str) -> Result<Target, String> {
    let internal = plumb::config::value("PLUMB_HOME").is_none()
        && plumb::config::value("PLUMB_GUARD_CONFIGURATION").is_some();
    if !internal && git::remote(root, "").is_ok_and(|remote| remote.host == DOMAIN) {
        return resolve(root, source);
    }
    let path = root.join("plumb.toml");
    let raw = std::fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let doc: toml::Table = raw
        .parse()
        .map_err(|error| format!("cannot parse {}: {error}", path.display()))?;
    let product = doc
        .get("release")
        .and_then(|held| held.get("product"))
        .and_then(toml::Value::as_str)
        .unwrap_or_default()
        .to_string();
    Ok(Target {
        product,
        authority: String::new(),
        derivatives: Vec::new(),
        profile: None,
    })
}

impl Root<'_> {
    fn manifested(&self) -> Result<Target, String> {
        let spec = super::release::Spec::read(&self.0.join("plumb.toml"))?;
        Ok(Target {
            product: spec.product,
            authority: spec.authority,
            derivatives: spec.depot.map_or_else(Vec::new, |depot| depot.derivatives),
            profile: None,
        })
    }
}

fn inline(raw: &str, identity: &str) -> Result<Target, String> {
    let catalog: Legacy = toml::from_str(raw)
        .map_err(|error| format!("cannot parse depot rules/products.toml: {error}"))?;
    catalog.validate()?;
    let product = catalog
        .product
        .into_iter()
        .find(|product| product.identity == identity)
        .ok_or_else(|| absent(identity))?;
    Ok(target(product.definition, None))
}

fn profiled<C: Configuration>(raw: &str, identity: &str, seat: &C) -> Result<Target, String> {
    let catalog: Catalog = toml::from_str(raw)
        .map_err(|error| format!("cannot parse depot rules/products.toml: {error}"))?;
    catalog.validate()?;
    let reference = catalog
        .product
        .into_iter()
        .find(|product| product.identity == identity)
        .ok_or_else(|| absent(identity))?;
    let path = format!("profiles/{}.toml", reference.profile);
    let raw = seat.read(&path, "")?;
    if plumb::depot::sha(raw.as_bytes()) != reference.profile {
        return Err(format!("product profile digest drift: {path}"));
    }
    let document: Document =
        toml::from_str(&raw).map_err(|error| format!("cannot parse {path}: {error}"))?;
    document.validate(&path)?;
    Ok(target(
        document.product,
        Some(Profile {
            configuration: seat
                .mark()
                .ok_or_else(|| "product profile has no configuration generation".to_string())?,
            digest: reference.profile,
            manifest: document.governance.manifest,
            ectropy: document.governance.ectropy,
        }),
    ))
}

fn target(product: Definition, profile: Option<Profile>) -> Target {
    Target {
        product: product.name,
        authority: product.authority,
        derivatives: product.derivatives,
        profile,
    }
}

impl Target {
    pub fn require(&self, derivative: Kind) -> Result<(), String> {
        if self.derivatives.contains(&derivative) {
            return Ok(());
        }
        Err(format!(
            "perish.code product {} does not carry the {} derivative",
            self.product,
            derivative.label()
        ))
    }
}

fn absent(identity: &str) -> String {
    format!("perish.code product identity {identity} is absent from the Plumb depot")
}
