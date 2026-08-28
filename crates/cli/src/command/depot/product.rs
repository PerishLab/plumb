use plumb::depot::v2::Kind;
use plumb::forgejo::git;
use plumb::rig::Rig;
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
struct Catalog {
    schema: String,
    #[serde(default)]
    product: Vec<Product>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Product {
    identity: String,
    name: String,
    authority: String,
    derivatives: Vec<Kind>,
}

pub fn resolve(root: &Path, rig: &Rig, derivative: Kind) -> Result<Target, String> {
    let remote = git::remote(root, "")?;
    let identity = format!("{}/{}/{}", remote.host, remote.owner, remote.repo);
    if remote.host == DOMAIN {
        let raw = super::held().read("rules/products.toml", FACTORY)?;
        return Catalog::parse(&raw)?.target(&identity, derivative, &rig.rules.source);
    }

    let spec = crate::shape::release::Spec::read(&root.join("plumb.toml"))?;
    let source = spec.derivative(derivative)?.source.clone();
    Ok(Target {
        product: spec.product,
        authority: spec.authority,
        source,
    })
}

impl Catalog {
    fn parse(raw: &str) -> Result<Self, String> {
        let held: Self = toml::from_str(raw)
            .map_err(|error| format!("cannot parse depot rules/products.toml: {error}"))?;
        if held.schema != "plumb.products/v1" {
            return Err(format!("unknown product catalog schema {}", held.schema));
        }
        let mut identities = BTreeSet::new();
        let mut names = BTreeSet::new();
        for product in &held.product {
            if product.identity.split('/').count() != 3
                || !product.identity.starts_with(&format!("{DOMAIN}/"))
                || product.identity.chars().any(char::is_whitespace)
            {
                return Err(format!(
                    "product identity {} is not one normalized {DOMAIN}/owner/repository identity",
                    product.identity
                ));
            }
            if !identities.insert(&product.identity) {
                return Err(format!("product identity {} is repeated", product.identity));
            }
            if product.name.is_empty() || product.name.chars().any(char::is_whitespace) {
                return Err(format!("product name {:?} is not one token", product.name));
            }
            if !names.insert(&product.name) {
                return Err(format!("product name {} is repeated", product.name));
            }
            if !product.authority.starts_with("https://")
                || product.authority.ends_with('/')
                || product.authority.chars().any(char::is_whitespace)
            {
                return Err(format!(
                    "product {} authority must be one normalized https URL",
                    product.name
                ));
            }
            if product.derivatives.is_empty() {
                return Err(format!("product {} declares no derivative", product.name));
            }
            let mut derivatives = BTreeSet::new();
            for derivative in &product.derivatives {
                if !derivatives.insert(*derivative) {
                    return Err(format!(
                        "product {} repeats the {} derivative",
                        product.name,
                        derivative.label()
                    ));
                }
            }
        }
        Ok(held)
    }

    fn target(&self, identity: &str, derivative: Kind, source: &str) -> Result<Target, String> {
        let product = self
            .product
            .iter()
            .find(|product| product.identity == identity)
            .ok_or_else(|| {
                format!("perish.code product identity {identity} is absent from the Plumb depot")
            })?;
        if !product.derivatives.contains(&derivative) {
            return Err(format!(
                "perish.code product {} does not carry the {} derivative",
                product.name,
                derivative.label()
            ));
        }
        Ok(Target {
            product: product.name.clone(),
            authority: product.authority.clone(),
            source: source.to_string(),
        })
    }
}
