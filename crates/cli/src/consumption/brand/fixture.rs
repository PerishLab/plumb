use super::{material, receipt, source};
use base64::Engine;
use sha2::{Digest, Sha512};
use std::io::Write;

pub(super) struct Fixture {
    pub(super) root: tempfile::TempDir,
    pub(super) materials: material::Materials,
    pub(super) app: source::App,
    pub(super) value: serde_json::Value,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let glyph = b"fixture glyph bytes";
        let raster = b"fixture raster bytes";
        let asset = |medium: &str, bytes: &[u8]| receipt::Asset {
            id: format!("fixture/{medium}"),
            product: "fixture".into(),
            kind: "shared".into(),
            form: "sign".into(),
            step: "full".into(),
            medium: medium.into(),
            surface: "light".into(),
            width: 100,
            height: 100,
            file: format!("media/fixture.{medium}"),
            digest: material::digest(bytes),
        };
        let assets = vec![asset("svg", glyph), asset("png", raster)];
        let model =
            serde_json::json!({"name":"fixture","digest":material::digest(b"fixture source")});
        let manifest = serde_json::to_vec(
            &serde_json::json!({"schema":"crest.assets/v1","model":model,"assets":assets}),
        )
        .unwrap();
        let package =
            serde_json::to_vec(&serde_json::json!({"name":"@perishlab/crest","version":"0.1.0"}))
                .unwrap();
        let files = vec![
            ("package.json", package.as_slice()),
            ("media/manifest.json", manifest.as_slice()),
            ("media/fixture.svg", glyph.as_slice()),
            ("media/fixture.png", raster.as_slice()),
        ];
        let payload = archive(files);
        let archive = root.path().join("fixture.tgz");
        std::fs::write(&archive, &payload).unwrap();
        let origin = plumb::packages::Origin {
            package: plumb::packages::Package {
                ecosystem: "npm".into(),
                name: "@perishlab/crest".into(),
                version: "0.1.0".into(),
            },
            integrity: format!(
                "sha512-{}",
                base64::engine::general_purpose::STANDARD.encode(Sha512::digest(&payload))
            ),
            archive,
        };
        let outputs = assets
            .iter()
            .map(|asset| {
                let mut value = serde_json::to_value(asset).unwrap();
                value["output"] = format!("/crest/a.{}", asset.medium).into();
                value
            })
            .collect::<Vec<_>>();
        let head = serde_json::json!({"product":"fixture","origin":"https://example.test"});
        let value = serde_json::json!({"schema":"design.brand/v1","package":{"name":"@perishlab/crest","version":"0.1.0"},"model":model,"manifest":{"file":"media/manifest.json","digest":material::digest(&manifest)},"assets":outputs,"head":head});
        let output = root.path().join("dist");
        std::fs::create_dir_all(output.join(".perish")).unwrap();
        std::fs::create_dir_all(output.join("crest")).unwrap();
        std::fs::write(output.join("crest/a.svg"), glyph).unwrap();
        std::fs::write(output.join("crest/a.png"), raster).unwrap();
        std::fs::write(output.join("index.html"),r#"<html><head><link rel="icon" href="/crest/a.svg"><link rel="apple-touch-icon" href="/crest/a.png"><meta property="og:image" content="https://example.test/crest/a.png"><meta name="twitter:card" content="summary"><meta name="twitter:image" content="https://example.test/crest/a.png"></head><body><img data-crest="fixture/svg" data-kind="shared" src="/crest/a.svg"></body></html>"#).unwrap();
        let held = Self {
            root,
            materials: material::Materials {
                origins: vec![origin],
                publications: Vec::new(),
            },
            app: source::App {
                path: "".into(),
                setup: crate::shape::web::Setup {
                    options: serde_json::json!({"brand":head}),
                    base: "/".into(),
                    output: "dist".into(),
                    problem: None,
                },
            },
            value,
        };
        held.write();
        held
    }

    pub(super) fn write(&self) {
        std::fs::write(
            self.root.path().join("dist/.perish/brand.json"),
            serde_json::to_vec(&self.value).unwrap(),
        )
        .unwrap();
    }

    pub(super) fn verify(&self) -> Result<usize, String> {
        receipt::Catalog::read(&self.materials)?.verify(
            self.root.path(),
            &self.app,
            &self.materials,
        )
    }
}

fn archive(files: Vec<(&str, &[u8])>) -> Vec<u8> {
    let writer = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    let mut archive = tar::Builder::new(writer);
    for (path, bytes) in files {
        let mut header = tar::Header::new_gnu();
        header.set_mode(0o644);
        header.set_size(bytes.len() as u64);
        header.set_cksum();
        archive
            .append_data(&mut header, format!("package/{path}"), bytes)
            .unwrap();
    }
    let mut writer = archive.into_inner().unwrap();
    writer.flush().unwrap();
    writer.finish().unwrap()
}
