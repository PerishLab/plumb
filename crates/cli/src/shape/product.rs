use plumb::depot::v3::Kind;
use plumb::forgejo::git;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::Path;

const DOMAIN: &str = "git.perish.top";
const FACTORY: &str = "schema = \"plumb.products/v1\"\n";

pub struct Target {
    pub product: String,
    pub authority: String,
    pub source: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Legacy {
    schema: String,
    #[serde(default)]
    product: Vec<Inline>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Inline {
    identity: String,
    #[serde(flatten)]
    definition: Definition,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    schema: String,
    #[serde(default)]
    product: Vec<Reference>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Reference {
    identity: String,
    profile: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: String,
    product: Definition,
    governance: Governance,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Definition {
    name: String,
    authority: String,
    derivatives: Vec<Kind>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Governance {
    manifest: String,
    ectropy: String,
}

struct Request<'a> {
    identity: &'a str,
    source: &'a str,
    derivative: Kind,
}

pub fn resolve(root: &Path, source: &str, derivative: Kind) -> Result<Target, String> {
    let remote = git::remote(root, "")?;
    if remote.host != DOMAIN {
        return manifested(root, derivative);
    }
    let identity = format!("{}/{}/{}", remote.host, remote.owner, remote.repo);
    let request = Request {
        identity: &identity,
        source,
        derivative,
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

fn manifested(root: &Path, derivative: Kind) -> Result<Target, String> {
    let spec = super::release::Spec::read(&root.join("plumb.toml"))?;
    let source = spec.derivative(derivative)?.source.clone();
    Ok(Target {
        product: spec.product,
        authority: spec.authority,
        source,
    })
}

fn inline(raw: &str, request: &Request<'_>) -> Result<Target, String> {
    let catalog: Legacy = toml::from_str(raw)
        .map_err(|error| format!("cannot parse depot rules/products.toml: {error}"))?;
    if catalog.schema != "plumb.products/v1" {
        return Err(format!("unknown product catalog schema {}", catalog.schema));
    }
    catalog.validate()?;
    let product = catalog
        .product
        .into_iter()
        .find(|product| product.identity == request.identity)
        .ok_or_else(|| absent(request.identity))?;
    target(product.definition, request)
}

fn profiled(
    raw: &str,
    request: &Request<'_>,
    seat: &crate::command::depot::Held,
) -> Result<Target, String> {
    let catalog: Catalog = toml::from_str(raw)
        .map_err(|error| format!("cannot parse depot rules/products.toml: {error}"))?;
    if catalog.schema != "plumb.products/v2" {
        return Err(format!("unknown product catalog schema {}", catalog.schema));
    }
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
    target(document.product, request)
}

fn target(product: Definition, request: &Request<'_>) -> Result<Target, String> {
    if !product.derivatives.contains(&request.derivative) {
        return Err(format!(
            "perish.code product {} does not carry the {} derivative",
            product.name,
            request.derivative.label()
        ));
    }
    Ok(Target {
        product: product.name,
        authority: product.authority,
        source: request.source.to_string(),
    })
}

impl Document {
    fn validate(&self, path: &str) -> Result<(), String> {
        if self.schema != "plumb.product-profile/v1" {
            return Err(format!("unknown product profile schema {}", self.schema));
        }
        token("product name", &self.product.name)?;
        authority(&self.product.name, &self.product.authority)?;
        derivatives(&self.product.name, &self.product.derivatives)?;
        self.governance
            .manifest
            .parse::<toml::Table>()
            .map_err(|error| format!("{path} carries an invalid governance manifest: {error}"))?;
        self.governance
            .ectropy
            .parse::<toml::Table>()
            .map_err(|error| format!("{path} carries an invalid Ectropy policy: {error}"))?;
        Ok(())
    }
}

impl Legacy {
    fn validate(&self) -> Result<(), String> {
        let mut identities = BTreeSet::new();
        let mut names = BTreeSet::new();
        for product in &self.product {
            identity(&product.identity, &mut identities)?;
            token("product name", &product.definition.name)?;
            if !names.insert(&product.definition.name) {
                return Err(format!(
                    "product name {} is repeated",
                    product.definition.name
                ));
            }
            authority(&product.definition.name, &product.definition.authority)?;
            derivatives(&product.definition.name, &product.definition.derivatives)?;
        }
        Ok(())
    }
}

impl Catalog {
    fn validate(&self) -> Result<(), String> {
        let mut identities = BTreeSet::new();
        for product in &self.product {
            identity(&product.identity, &mut identities)?;
            if product.profile.len() != 64
                || !product.profile.bytes().all(|byte| byte.is_ascii_hexdigit())
            {
                return Err(format!(
                    "product identity {} has an invalid profile digest",
                    product.identity
                ));
            }
        }
        Ok(())
    }
}

fn identity(identity: &str, identities: &mut BTreeSet<String>) -> Result<(), String> {
    if identity.split('/').count() != 3
        || !identity.starts_with(&format!("{DOMAIN}/"))
        || identity.chars().any(char::is_whitespace)
    {
        return Err(format!(
            "product identity {identity} is not one normalized {DOMAIN}/owner/repository identity"
        ));
    }
    if !identities.insert(identity.to_string()) {
        return Err(format!("product identity {identity} is repeated"));
    }
    Ok(())
}

fn token(subject: &str, value: &str) -> Result<(), String> {
    let mut bytes = value.bytes();
    if bytes.next().is_none_or(|byte| !byte.is_ascii_lowercase())
        || !bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(format!("{subject} {value:?} is not one token"));
    }
    Ok(())
}

fn authority(product: &str, authority: &str) -> Result<(), String> {
    if !authority.starts_with("https://")
        || authority.ends_with('/')
        || authority.chars().any(char::is_whitespace)
    {
        return Err(format!(
            "product {product} authority must be one normalized https URL"
        ));
    }
    Ok(())
}

fn derivatives(product: &str, derivatives: &[Kind]) -> Result<(), String> {
    if derivatives.is_empty() {
        return Err(format!("product {product} declares no derivative"));
    }
    let mut held = BTreeSet::new();
    for derivative in derivatives {
        if !held.insert(*derivative) {
            return Err(format!(
                "product {product} repeats the {} derivative",
                derivative.label()
            ));
        }
    }
    Ok(())
}

fn absent(identity: &str) -> String {
    format!("perish.code product identity {identity} is absent from the Plumb depot")
}
