use plumb::depot::v3::Kind;
use plumb::forgejo::git;
use std::path::Path;

pub(crate) use super::repository::product::Source;
use super::repository::product::{Catalog, Definition, Document, Legacy, Migration, Migrations};

const DOMAIN: &str = "git.perish.top";
const FACTORY: &str = "schema = \"plumb.products/v1\"\n";
const MIGRATIONS: &str = "schema = \"plumb.migrations/v1\"\n";

#[path = "repository/product/catalog.rs"]
mod catalog;

pub fn names() -> Result<Vec<String>, String> {
    catalog::names(&crate::command::depot::held())
}

pub struct Target {
    pub product: String,
    pub authority: String,
    pub depot: String,
    derivatives: Vec<Kind>,
    pub profile: Option<Profile>,
}

pub struct Profile {
    pub configuration: String,
    pub digest: String,
    pub manifest: String,
    pub ectropy: String,
    pub source: Source,
}

pub fn governance(root: &Path) -> Result<Option<Target>, String> {
    match git::remote(root, "") {
        Ok(remote) if remote.host == DOMAIN => {
            let rig = plumb::rig::Rig::resolve(None).map_err(|error| error.to_string())?;
            let mut target = resolve(root, &rig.rules.source)?;
            if projected()
                && let Some(profile) = target.profile.as_mut()
            {
                profile.source = Source::Repository;
            }
            Ok(Some(target))
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
        "plumb.products/v1" => inline(&raw, &identity, seat),
        "plumb.products/v2" => profiled(&raw, &identity, seat),
        _ => Err(format!("unknown product catalog schema {schema}")),
    }
}

pub fn guard(root: &Path, source: &str) -> Result<Target, String> {
    if let Ok(remote) = git::remote(root, "")
        && remote.host == DOMAIN
    {
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
        depot: String::new(),
        derivatives: Vec::new(),
        profile: None,
    })
}

fn projected() -> bool {
    plumb::config::value("PLUMB_HOME").is_none()
        && plumb::config::value("PLUMB_GUARD_VIEW").is_some()
        && binding()
}

fn binding() -> bool {
    plumb::config::value("PLUMB_GUARD_CONFIGURATION").is_some()
        || plumb::config::value("PLUMB_GUARD_DEPOT").is_some()
}

impl Root<'_> {
    fn manifested(&self) -> Result<Target, String> {
        let spec = super::release::Spec::read(&self.0.join("plumb.toml"))?;
        Ok(Target {
            product: spec.product,
            authority: spec.authority,
            depot: spec
                .depot
                .as_ref()
                .map(|depot| depot.source.clone())
                .unwrap_or_default(),
            derivatives: spec.depot.map_or_else(Vec::new, |depot| depot.derivatives),
            profile: None,
        })
    }
}

fn inline<C: Configuration>(raw: &str, identity: &str, seat: &C) -> Result<Target, String> {
    let catalog: Legacy = toml::from_str(raw)
        .map_err(|error| format!("cannot parse depot rules/products.toml: {error}"))?;
    catalog.validate()?;
    let product = catalog
        .product
        .into_iter()
        .find(|product| product.identity == identity)
        .ok_or_else(|| absent(identity))?;
    let migrations = seat.read("rules/migrations.toml", MIGRATIONS)?;
    let migrations: Migrations = toml::from_str(&migrations)
        .map_err(|error| format!("cannot parse depot rules/migrations.toml: {error}"))?;
    migrations.validate()?;
    let migration = migrations
        .product
        .into_iter()
        .find(|migration| migration.identity == identity);
    match migration {
        Some(migration) => migrated(seat, product.definition, migration),
        None => target(product.definition, None),
    }
}

fn migrated<C: Configuration>(
    seat: &C,
    definition: Definition,
    migration: Migration,
) -> Result<Target, String> {
    let path = format!("profiles/{}.toml", migration.profile);
    let raw = seat.read(&path, "")?;
    if plumb::depot::sha(raw.as_bytes()) != migration.profile {
        return Err(format!("product profile digest drift: {path}"));
    }
    let document: Document =
        toml::from_str(&raw).map_err(|error| format!("cannot parse {path}: {error}"))?;
    document.validate(&path)?;
    if document.product.identity() != definition.identity() {
        return Err(format!(
            "product migration profile {} differs from its catalog definition",
            migration.profile
        ));
    }
    target(
        document.product,
        Some(Profile {
            configuration: seat
                .mark()
                .ok_or_else(|| "product profile has no configuration generation".to_string())?,
            digest: migration.profile,
            manifest: document.governance.manifest,
            ectropy: document.governance.ectropy,
            source: migration.source,
        }),
    )
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
    target(
        document.product,
        Some(Profile {
            configuration: seat
                .mark()
                .ok_or_else(|| "product profile has no configuration generation".to_string())?,
            digest: reference.profile,
            manifest: document.governance.manifest,
            ectropy: document.governance.ectropy,
            source: Source::Depot,
        }),
    )
}

fn target(product: Definition, profile: Option<Profile>) -> Result<Target, String> {
    let depot = super::repository::product::depot(
        &product.name,
        &product.authority,
        product.depot.as_deref(),
    )?;
    Ok(Target {
        product: product.name,
        authority: product.authority,
        depot,
        derivatives: product.derivatives,
        profile,
    })
}

impl Target {
    pub fn derivatives(&self) -> &[Kind] {
        &self.derivatives
    }
}

fn absent(identity: &str) -> String {
    format!("perish.code product identity {identity} is absent from the Plumb depot")
}

#[cfg(test)]
mod tests {
    use super::{Profile, Source, Target};
    use crate::shape::release::Spec;

    const MANIFEST: &str = "[release]\nproduct = \"probe\"\nauthority = \"https://releases.probe.test\"\nbinaries = [\"probe\"]\ntargets = [\"x86_64-unknown-linux-gnu\"]\n";

    fn target(product: &str, authority: &str) -> Target {
        Target {
            product: product.into(),
            authority: authority.into(),
            depot: "https://depot.probe.test".into(),
            derivatives: Vec::new(),
            profile: Some(Profile {
                configuration: "a".repeat(64),
                digest: "b".repeat(64),
                manifest: MANIFEST.into(),
                ectropy: String::new(),
                source: Source::Depot,
            }),
        }
    }

    #[test]
    fn a_governed_spec_must_match_the_identity_its_product_declares() {
        let root = tempfile::tempdir().expect("root");
        let held = target("probe", "https://releases.probe.test");
        let spec = Spec::governed(root.path(), held).expect("governed spec");
        assert_eq!(spec.product, "probe");
        assert_eq!(spec.profile.as_deref(), Some("b".repeat(64).as_str()));
        for drifted in [
            target("other", "https://releases.probe.test"),
            target("probe", "https://releases.other.test"),
        ] {
            let refusal = Spec::governed(root.path(), drifted).expect_err("identity drift");
            assert!(
                refusal.contains("release identity differs from its product definition"),
                "{refusal}"
            );
        }
    }
}
