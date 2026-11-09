use super::material::{Materials, digest, regular, unique};
use super::source::App;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Deserialize, Serialize, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct Asset {
    pub(super) id: String,
    pub(super) product: String,
    pub(super) kind: String,
    pub(super) form: String,
    pub(super) step: String,
    pub(super) medium: String,
    pub(super) surface: String,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) file: String,
    pub(super) digest: String,
}

#[derive(Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Model {
    name: String,
    digest: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: String,
    model: Model,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Package {
    name: String,
    version: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    file: String,
    digest: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    schema: String,
    package: Package,
    model: Model,
    manifest: Input,
    assets: Vec<Output>,
    head: Option<Selection>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Output {
    id: String,
    product: String,
    kind: String,
    form: String,
    step: String,
    medium: String,
    surface: String,
    width: u32,
    height: u32,
    file: String,
    digest: String,
    output: String,
}
impl Output {
    fn asset(&self) -> Asset {
        Asset {
            id: self.id.clone(),
            product: self.product.clone(),
            kind: self.kind.clone(),
            form: self.form.clone(),
            step: self.step.clone(),
            medium: self.medium.clone(),
            surface: self.surface.clone(),
            width: self.width,
            height: self.height,
            file: self.file.clone(),
            digest: self.digest.clone(),
        }
    }
}

#[derive(Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Selection {
    pub(super) product: String,
    pub(super) origin: String,
}

pub(super) struct Catalog {
    pub(super) assets: BTreeMap<String, Asset>,
    pub(super) files: BTreeMap<String, Vec<u8>>,
    model: Model,
    digest: String,
}

impl Catalog {
    pub(super) fn read(materials: &Materials) -> Result<Self, String> {
        let files = materials.archive("@perishlab/crest")?.files;
        let bytes = files
            .get("media/manifest.json")
            .ok_or("formal Crest package omits media manifest")?;
        let manifest: Manifest = serde_json::from_slice(bytes)
            .map_err(|error| format!("formal Crest manifest is unqualified: {error}"))?;
        if manifest.schema != "crest.assets/v1" {
            return Err("Crest manifest schema is unsupported".into());
        }
        let assets = unique(
            manifest
                .assets
                .into_iter()
                .map(|asset| (asset.id.clone(), asset))
                .collect(),
        )?;
        if assets.is_empty() {
            return Err("formal Crest manifest is empty".into());
        }
        for asset in assets.values() {
            super::source::relative(&asset.file)?;
            let image = files
                .get(&asset.file)
                .ok_or("formal Crest image is missing")?;
            if digest(image) != asset.digest {
                return Err("formal Crest image differs from its manifest".into());
            }
        }
        Ok(Self {
            assets,
            model: manifest.model,
            digest: digest(bytes),
            files,
        })
    }

    pub(super) fn verify(
        &self,
        root: &Path,
        app: &App,
        materials: &Materials,
    ) -> Result<usize, String> {
        let directory = root.join(&app.path).join(&app.setup.output);
        let bytes = regular(&directory, ".perish/brand.json")?;
        let receipt: Receipt = serde_json::from_slice(&bytes)
            .map_err(|error| format!("brand output receipt is unqualified: {error}"))?;
        let origin = materials.origin("@perishlab/crest")?;
        if receipt.schema != "design.brand/v1"
            || receipt.package.name != origin.package.name
            || receipt.package.version != origin.package.version
        {
            return Err("brand receipt differs from captured formal package identity".into());
        }
        if receipt.model != self.model
            || receipt.manifest.file != "media/manifest.json"
            || receipt.manifest.digest != self.digest
        {
            return Err("brand receipt differs from the formal model/manifest".into());
        }
        let expected: Selection = serde_json::from_value(app.setup.options["brand"].clone())
            .map_err(|error| format!("head declaration is unqualified: {error}"))?;
        if receipt.head.as_ref() != Some(&expected) {
            return Err("head receipt differs from the static plugin declaration".into());
        }
        let outputs = unique(
            receipt
                .assets
                .into_iter()
                .map(|output| (output.id.clone(), output))
                .collect(),
        )?;
        if outputs.len() != self.assets.len() {
            return Err("brand receipt is incomplete or surplus".into());
        }
        for (id, asset) in &self.assets {
            let output = outputs
                .get(id)
                .ok_or("brand receipt omits a formal selection")?;
            if output.asset() != *asset {
                return Err(format!("brand output {id} changed its formal descriptor"));
            }
            let path = output
                .output
                .strip_prefix(&app.setup.base)
                .ok_or("brand output is outside its Vite base")?;
            if path.contains(['%', '?', '#', ':']) {
                return Err("encoded or external brand output is unqualified".into());
            }
            let actual = regular(&directory, path)?;
            if self.files.get(&asset.file) != Some(&actual) {
                return Err(format!(
                    "brand output {path} differs from the formal image bytes"
                ));
            }
        }
        self.product(&expected.product)?;
        let mut head = BTreeMap::<String, Vec<Asset>>::new();
        for output in outputs.values() {
            head.entry(output.output.clone())
                .or_default()
                .push(output.asset());
        }
        super::head::verify(&directory, &expected, &head)?;
        Ok(outputs.len())
    }

    pub(super) fn product(&self, product: &str) -> Result<(), String> {
        if !self.assets.values().any(|asset| asset.product == product) {
            return Err(format!("product {product:?} has no formal Crest assets"));
        }
        Ok(())
    }
}
