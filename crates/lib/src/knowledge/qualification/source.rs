use super::{
    Fault, Read, Result,
    wire::{LIMIT, POLICY, WHARF, Wire},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use sha1::{Digest, Sha1};

pub(super) struct Source<'a, R>(pub &'a R);

pub(super) struct Subject {
    pub source: Value,
    pub tag: Value,
    pub declaration: Value,
    pub repository: Value,
    pub authority: String,
    pub channel: String,
    pub product: String,
    pub installed: Vec<String>,
}

impl<R: Read> Source<'_, R> {
    fn repository(&self, name: &str) -> Result<Value> {
        let value = Wire::document(
            self.0,
            &format!("https://api.github.com/repos/{name}"),
            false,
        )?;
        Wire::matches(
            &value,
            &json!({"full_name":name, "visibility":"public", "archived":false, "disabled":false}),
            "repository authority differs",
        )?;
        Wire::require(
            value["owner"]["id"] == 330236468_u64
                && value["owner"]["node_id"] == "O_kgDOE68CNA"
                && value["owner"]["login"] == "PerishLab",
            "repository owner differs",
        )?;
        Wire::require(
            Wire::number(&value["id"])? > 0,
            "repository numeric identity is absent",
        )?;
        Wire::text(&value["node_id"])?;
        Ok(
            json!({"id":value["id"], "node":value["node_id"], "name":name, "default":value["default_branch"]}),
        )
    }

    fn head(&self) -> Result<String> {
        let value = Wire::document(self.0, &format!("{WHARF}/branches/main"), false)?;
        Wire::require(
            value["name"] == "main" && value["protected"] == true,
            "requirements need protected main",
        )?;
        Ok(Wire::hex(&value["commit"]["sha"], 40)?.into())
    }

    pub fn content(&self, repository: &str, commit: &str, path: &str) -> Result<Vec<u8>> {
        let url = format!("https://api.github.com/repos/{repository}/contents/{path}?ref={commit}");
        let value = Wire::document(self.0, &url, false)?;
        Wire::require(
            value["type"] == "file" && value["path"] == path && value["encoding"] == "base64",
            "source content identity differs",
        )?;
        let size = Wire::number(&value["size"])?;
        Wire::require(
            size > 0 && size <= (LIMIT / 2) as u64,
            "source content exceeds bounds",
        )?;
        let text = Wire::text(&value["content"])?;
        Wire::require(text.len() <= LIMIT, "encoded source exceeds bounds")?;
        let encoded = text.lines().collect::<String>();
        let raw = STANDARD
            .decode(encoded)
            .map_err(|_| Fault::refused("invalid source base64"))?;
        let mut hash = Sha1::new();
        hash.update(format!("blob {}\0", raw.len()).as_bytes());
        hash.update(&raw);
        Wire::require(
            raw.len() as u64 == size
                && format!("{:x}", hash.finalize()) == Wire::hex(&value["sha"], 40)?,
            "source differs from its native Git blob",
        )?;
        Ok(raw)
    }

    pub fn policy(&self) -> Result<Value> {
        let repository = self.repository("PerishLab/wharf")?;
        Wire::require(
            repository["id"] == 1374358737_u64
                && repository["node"] == "R_kgDOUesM0Q"
                && repository["default"] == "main",
            "foreign requirements repository",
        )?;
        let head = self.head()?;
        let commit = Wire::document(self.0, &format!("{WHARF}/git/commits/{head}"), false)?;
        Wire::require(
            commit["sha"] == head,
            "requirements commit differs from main",
        )?;
        Wire::hex(&commit["tree"]["sha"], 40)?;
        let raw = self.content("PerishLab/wharf", &head, "resources/check/windtun.json")?;
        Wire::require(raw.len() <= 4096, "requirements descriptor exceeds bounds")?;
        let value = Wire::parse(&raw, true)?;
        Wire::fields(&value, &["schema", "policy", "implementation"])?;
        Wire::require(
            value["schema"] == "wharf.windtun.requirements/v1" && value["policy"] == POLICY,
            "unsupported current requirements",
        )?;
        Wire::hex(&value["implementation"], 64)?;
        Wire::require(
            self.head()? == head && self.repository("PerishLab/wharf")? == repository,
            "requirements source moved during readback",
        )?;
        Ok(
            json!({"requirements":value, "repository":repository, "commit":head, "tree":commit["tree"]["sha"], "sha256":Wire::sha(&raw)}),
        )
    }

    fn commit(&self, repository: &str, marker: &str) -> Result<Value> {
        let root = format!("https://api.github.com/repos/{repository}");
        let reference = Wire::document(self.0, &format!("{root}/git/ref/tags/{marker}"), false)?;
        Wire::require(
            reference["ref"] == format!("refs/tags/{marker}")
                && reference["object"]["type"] == "tag",
            "release marker is not the exact annotated tag",
        )?;
        let tag = Wire::hex(&reference["object"]["sha"], 40)?;
        let annotated = Wire::document(self.0, &format!("{root}/git/tags/{tag}"), false)?;
        Wire::require(
            annotated["sha"] == tag && annotated["tag"] == marker,
            "annotated marker identity differs",
        )?;
        Wire::require(
            annotated["object"]["type"] == "commit",
            "release tag does not name a commit",
        )?;
        let commit = Wire::hex(&annotated["object"]["sha"], 40)?;
        let value = Wire::document(self.0, &format!("{root}/git/commits/{commit}"), false)?;
        Wire::require(value["sha"] == commit, "release commit differs from tag")?;
        Wire::hex(&value["tree"]["sha"], 40)?;
        Ok(
            json!({"source":{"repository":repository, "marker":marker, "commit":commit, "tree":value["tree"]["sha"]}, "tag":tag}),
        )
    }

    pub fn subject(&self, repository: &str, marker: &str) -> Result<Subject> {
        let product = repository
            .strip_prefix("PerishLab/")
            .ok_or_else(|| Fault::refused("subject requires PerishLab repository"))?;
        Wire::names(&json!([product]))?;
        let version = semver::Version::parse(marker.strip_prefix('v').unwrap_or_default())
            .map_err(|_| Fault::refused("invalid release marker"))?;
        let (channel, number) = version
            .pre
            .as_str()
            .split_once('.')
            .ok_or_else(|| Fault::refused("qualification selects a non-stable marker"))?;
        Wire::matches(
            &json!({"canonical":format!("v{version}"), "build":version.build.as_str()}),
            &json!({"canonical":marker, "build":""}),
            "release marker is not canonical",
        )?;
        match (channel, number.parse::<u64>()) {
            ("alpha" | "beta" | "rc", Ok(1..)) => {}
            _ => return Err(Fault::refused("unsupported non-stable marker")),
        }
        let identity = self.repository(repository)?;
        let resolved = self.commit(repository, marker)?;
        let source = resolved["source"].clone();
        let raw = self.content(repository, Wire::text(&source["commit"])?, "plumb.toml")?;
        let text =
            std::str::from_utf8(&raw).map_err(|_| Fault::refused("manifest is not UTF-8"))?;
        let manifest: toml::Value =
            toml::from_str(text).map_err(|_| Fault::refused("invalid product manifest"))?;
        Wire::require(
            manifest
                .get("identity")
                .and_then(|v| v.get("ns"))
                .and_then(|v| v.as_str())
                == Some(product),
            "manifest product differs from selected repository",
        )?;
        let release = manifest
            .get("release")
            .ok_or_else(|| Fault::refused("missing release declaration"))?;
        let authority = release
            .get("authority")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Fault::refused("missing release authority"))?
            .to_string();
        Wire::authority(&authority)?;
        let specification = super::super::parse(text)
            .map_err(Fault::refused)?
            .ok_or_else(|| Fault::refused("product has no Windtun declaration"))?;
        let declaration = json!({"schema":"wharf.trial.declaration/v1", "source":source, "manifest":Wire::sha(&raw), "authority":authority, "specification":specification});
        Ok(Subject {
            source,
            tag: resolved["tag"].clone(),
            declaration,
            repository: identity,
            authority,
            channel: channel.into(),
            product: product.into(),
            installed: Self::installed(release)?,
        })
    }

    fn installed(release: &toml::Value) -> Result<Vec<String>> {
        let binaries = release
            .get("binaries")
            .and_then(|v| v.as_array())
            .ok_or_else(|| Fault::refused("missing binary inventory"))?;
        let mut names = Vec::new();
        for binary in binaries {
            let name = binary
                .as_str()
                .ok_or_else(|| Fault::refused("invalid binary name"))?;
            let options = release.get("binary").and_then(|v| v.get(name));
            let install = options
                .and_then(|v| v.get("install"))
                .map(|v| {
                    v.as_bool()
                        .ok_or_else(|| Fault::refused("invalid binary install flag"))
                })
                .transpose()?
                .unwrap_or(true);
            let targets = options
                .and_then(|v| v.get("targets"))
                .or_else(|| release.get("targets"))
                .and_then(|v| v.as_array())
                .ok_or_else(|| Fault::refused("missing binary targets"))?;
            if install
                && targets
                    .iter()
                    .any(|v| v.as_str() == Some("x86_64-unknown-linux-gnu"))
            {
                names.push(name.to_string());
            }
        }
        Wire::names(&json!(names))?;
        Ok(names)
    }

    pub fn unchanged(&self, subject: &Subject) -> Result<()> {
        let source = &subject.source;
        let same = self.commit(
            Wire::text(&source["repository"])?,
            Wire::text(&source["marker"])?,
        )? == json!({"source":source, "tag":subject.tag});
        Wire::require(
            same && self.repository(Wire::text(&source["repository"])?)? == subject.repository,
            "subject source or authority moved during readback",
        )
    }
}
