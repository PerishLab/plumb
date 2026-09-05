use plumb::depot::v3::Kind;
use serde::Deserialize;
use std::collections::BTreeSet;

const FORGE: &str = "git.perish.top";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::shape) struct Legacy {
    schema: String,
    #[serde(default)]
    pub product: Vec<Inline>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::shape) struct Inline {
    pub identity: String,
    #[serde(flatten)]
    pub definition: Definition,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::shape) struct Catalog {
    schema: String,
    #[serde(default)]
    pub product: Vec<Reference>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Source {
    Repository,
    Depot,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::shape) struct Reference {
    pub identity: String,
    pub profile: String,
    pub source: Option<Source>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::shape) struct Document {
    schema: String,
    pub product: Definition,
    pub governance: Governance,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::shape) struct Definition {
    pub name: String,
    pub authority: String,
    pub derivatives: Vec<Kind>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::shape) struct Governance {
    pub manifest: String,
    pub ectropy: String,
}

impl Document {
    pub fn validate(&self, path: &str) -> Result<(), String> {
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
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != "plumb.products/v1" {
            return Err(format!("unknown product catalog schema {}", self.schema));
        }
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
    pub fn validate(&self) -> Result<(), String> {
        if !matches!(
            self.schema.as_str(),
            "plumb.products/v2" | "plumb.products/v3"
        ) {
            return Err(format!("unknown product catalog schema {}", self.schema));
        }
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
            if self.schema == "plumb.products/v3" && product.source.is_none() {
                return Err(format!(
                    "product identity {} names no governance source",
                    product.identity
                ));
            }
        }
        Ok(())
    }
}

fn identity(identity: &str, identities: &mut BTreeSet<String>) -> Result<(), String> {
    if identity.split('/').count() != 3
        || !identity.starts_with(&format!("{FORGE}/"))
        || identity.chars().any(char::is_whitespace)
    {
        return Err(format!(
            "product identity {identity} is not one normalized {FORGE}/owner/repository identity"
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
