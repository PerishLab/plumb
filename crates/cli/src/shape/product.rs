use plumb::depot::v3::Kind;
use plumb::forgejo::git;
use std::path::Path;

use super::repository::product::{Catalog, Definition, Document, Legacy};

const DOMAIN: &str = "git.perish.top";
const FACTORY: &str = "schema = \"plumb.products/v1\"\n";

pub struct Target {
    pub product: String,
    pub authority: String,
    pub source: String,
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

struct Request<'a> {
    identity: &'a str,
    source: &'a str,
}

struct Root<'a>(&'a Path);

pub fn resolve(root: &Path, source: &str) -> Result<Target, String> {
    let remote = git::remote(root, "")?;
    if remote.host != DOMAIN {
        return Root(root).manifested();
    }
    let identity = format!("{}/{}/{}", remote.host, remote.owner, remote.repo);
    let request = Request {
        identity: &identity,
        source,
    };
    let seat = super::super::command::depot::held();
    let raw = seat.read("rules/products.toml", FACTORY)?;
    let schema = raw
        .parse::<toml::Table>()
        .ok()
        .and_then(|held| held.get("schema")?.as_str().map(str::to_string))
        .ok_or_else(|| "depot rules/products.toml names no schema".to_string())?;
    match schema.as_str() {
        "plumb.products/v1" => inline(&raw, &request),
        "plumb.products/v2" => profiled(&raw, &request, &seat),
        _ => Err(format!("unknown product catalog schema {schema}")),
    }
}

pub fn guard(root: &Path, source: &str) -> Result<Target, String> {
    if git::remote(root, "").is_ok_and(|remote| remote.host == DOMAIN) {
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
        source: String::new(),
        derivatives: Vec::new(),
        profile: None,
    })
}

impl Root<'_> {
    fn manifested(&self) -> Result<Target, String> {
        let spec = super::release::Spec::read(&self.0.join("plumb.toml"))?;
        let depot = spec
            .depot
            .as_ref()
            .ok_or_else(|| "release declares no depot".to_string())?;
        Ok(Target {
            product: spec.product,
            authority: spec.authority,
            source: depot.source.clone(),
            derivatives: depot.derivatives.clone(),
            profile: None,
        })
    }
}

fn inline(raw: &str, request: &Request<'_>) -> Result<Target, String> {
    let catalog: Legacy = toml::from_str(raw)
        .map_err(|error| format!("cannot parse depot rules/products.toml: {error}"))?;
    catalog.validate()?;
    let product = catalog
        .product
        .into_iter()
        .find(|product| product.identity == request.identity)
        .ok_or_else(|| absent(request.identity))?;
    Ok(target(product.definition, request, None))
}

fn profiled(
    raw: &str,
    request: &Request<'_>,
    seat: &crate::command::depot::Held,
) -> Result<Target, String> {
    let catalog: Catalog = toml::from_str(raw)
        .map_err(|error| format!("cannot parse depot rules/products.toml: {error}"))?;
    catalog.validate()?;
    let reference = catalog
        .product
        .into_iter()
        .find(|product| product.identity == request.identity)
        .ok_or_else(|| absent(request.identity))?;
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
        request,
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

fn target(product: Definition, request: &Request<'_>, profile: Option<Profile>) -> Target {
    Target {
        product: product.name,
        authority: product.authority,
        source: request.source.to_string(),
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
