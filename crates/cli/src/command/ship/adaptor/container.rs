use super::image::Image;
use crate::command::ship::package::oci;
use crate::command::ship::package::production::Proof;
use crate::command::ship::package::project::{Source, fetch as fetch_workload};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Request<'a> {
    pub version: &'a str,
    pub commit: &'a str,
    pub artifacts: &'a Path,
    pub credential: &'a str,
    pub reuse: &'a str,
    pub(in crate::command::ship) proof: Option<Proof<'a>>,
}

pub fn run(carrier: &Image<'_>, request: Request<'_>) -> Result<String, String> {
    let source = Source::parse(request.reuse)?;
    let proof = request.proof.as_ref();
    let producer = proof
        .filter(|_| source.kind == "none")
        .map(|proof| proof.contract.start(&carrier.spec.root))
        .transpose()?;
    let scoped = Image {
        spec: carrier.spec,
        execution: producer
            .as_ref()
            .map(plumb::rule::Producer::execution)
            .or(carrier.execution),
    };
    let workload = scoped.prepare(&request)?;
    let receipt = match producer {
        Some(producer) => Some(producer.finish(&workload)?),
        None => proof.and_then(|proof| proof.receipt).cloned(),
    };
    if let Some(receipt) = &receipt {
        proof
            .ok_or("image receipt has no production contract")?
            .contract
            .verify(receipt)?;
    }
    let publisher = proof
        .map(|_| crate::command::ship::package::registry::publisher(&carrier.spec.root))
        .transpose()?;
    let image = oci::Image::read(
        carrier.spec,
        &workload,
        publisher.as_ref().or(carrier.execution),
    )?;
    let publication = image.publish(carrier.spec, request.credential)?;
    let output = serde_json::json!({
        "format": "plumb.image-project/v1", "version": request.version,
        "workload": workload, "publication": publication, "provenance": image.provenance,
        "receipt": receipt,
    });
    serde_json::to_string(&output).map_err(|error| format!("cannot encode image project: {error}"))
}

impl Image<'_> {
    fn prepare(&self, request: &Request<'_>) -> Result<PathBuf, String> {
        let declared = self.spec.oci.as_ref().ok_or("image declares no registry")?;
        let source = Source::parse(request.reuse)?;
        let reference = reference(declared, request.version);
        if source.kind == "workload"
            && let Some(proof) = &request.proof
        {
            proof.contract.verify(
                proof
                    .receipt
                    .ok_or("reused image has no production receipt")?,
            )?;
        }
        let workload = match source.kind.as_str() {
            "none" => {
                self.build(request.version, request.commit, request.artifacts)?;
                self.save(&reference)?
            }
            "workload" => {
                let path = self.workload()?;
                let bytes = fetch_workload("image", &source.source)?;
                std::fs::write(&path, bytes)
                    .map_err(|error| format!("cannot stage image workload: {error}"))?;
                path
            }
            "url" => return Err("a held publication URL must skip the image action".into()),
            _ => unreachable!(),
        };
        if source.kind == "workload"
            && let Some(proof) = &request.proof
        {
            let receipt = proof
                .receipt
                .ok_or("reused image has no production receipt")?;
            receipt.verify(&workload)?;
        }
        Ok(workload)
    }

    pub fn publish(&self, version: &str, credential: &str) -> Result<String, String> {
        let Some(declared) = &self.spec.oci else {
            return Ok(format!("{} has no image attachment", self.spec.product));
        };
        let archive = self.save(&reference(declared, version))?;
        let image = oci::Image::read(self.spec, &archive, self.execution)?;
        let publication = image.publish(self.spec, credential)?;
        Ok(format!("published {} as {publication}", self.spec.product))
    }

    pub(super) fn workload(&self) -> Result<PathBuf, String> {
        let seat = self.spec.root.join("target/image");
        std::fs::create_dir_all(&seat)
            .map_err(|error| format!("cannot open {}: {error}", seat.display()))?;
        Ok(seat.join(format!("{}-image.tar", self.spec.product)))
    }
}

pub(super) fn hex(value: &str, width: usize) -> bool {
    value.len() == width
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(super) fn reference(oci: &crate::shape::release::Oci, version: &str) -> String {
    format!("{}/{}:{version}", oci.registry, oci.image)
}

pub(super) fn fetch(url: &str, path: &Path) -> Result<(), String> {
    let status = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--retry",
            "3",
            "--output",
        ])
        .arg(path)
        .arg(url)
        .status()
        .map_err(|error| format!("cannot run curl: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("cannot read the published payload {url}"))
    }
}
