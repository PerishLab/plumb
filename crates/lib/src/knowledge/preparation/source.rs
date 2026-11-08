use super::super::{Point, World, wire::Wire};
use super::tree::Tree;
use crate::windtun::qualification::Fault;
use base64::{Engine, engine::general_purpose::STANDARD};
use flate2::bufread::GzDecoder;
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Read};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    schema: String,
    source: Point,
    head: String,
    tree: String,
    files: usize,
    payload: Packed,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Packed {
    encoding: String,
    sha256: String,
    size: usize,
    data: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct File {
    pub path: String,
    pub mode: String,
    pub oid: String,
    pub size: usize,
    pub sha256: String,
    pub data: String,
}

#[derive(Debug, Serialize)]
pub struct Payload {
    pub source: Point,
    pub head: String,
    pub tree: String,
    pub sha256: String,
    pub files: Vec<File>,
}

impl Payload {
    const TOTAL: usize = 32 * 1024 * 1024;

    pub fn parse(raw: &[u8], world: &World) -> Result<Self, Fault> {
        world.validate()?;
        let source: Source = Wire::parse(raw)?;
        Wire::require(
            source.schema == "wharf.release.source/v1",
            "unsupported effective source schema",
        )?;
        Wire::require(
            source.source == world.source && source.head == world.head && source.tree == world.tree,
            "complete effective source names another world",
        )?;
        Wire::require(
            (1..=4096).contains(&source.files),
            "effective source file count exceeds bounds",
        )?;
        let files = Self::unpack(&source.payload)?;
        Wire::require(
            files.len() == source.files,
            "effective source file count differs",
        )?;
        Self::files(&files)?;
        Wire::require(
            Tree::digest(&files)? == world.tree,
            "complete effective source Git tree differs",
        )?;
        Ok(Self {
            source: source.source,
            head: source.head,
            tree: source.tree,
            sha256: Wire::sha(raw),
            files,
        })
    }

    fn unpack(packed: &Packed) -> Result<Vec<File>, Fault> {
        Wire::require(
            packed.encoding == "gzip+base64" && packed.size > 0 && packed.size <= Self::TOTAL * 2,
            "effective source encoding or expanded size exceeds bounds",
        )?;
        Wire::hex(&packed.sha256, 64)?;
        let compressed = STANDARD
            .decode(&packed.data)
            .map_err(|_| Fault::refused("invalid effective source compression encoding"))?;
        let mut decoder = GzDecoder::new(Cursor::new(&compressed));
        let mut raw = Vec::new();
        decoder
            .by_ref()
            .take((packed.size + 1) as u64)
            .read_to_end(&mut raw)
            .map_err(|_| Fault::refused("invalid effective source compressed bytes"))?;
        Wire::require(
            raw.len() == packed.size && Wire::sha(&raw) == packed.sha256,
            "effective source expanded bytes differ",
        )?;
        Wire::require(
            decoder.into_inner().position() == compressed.len() as u64,
            "effective source compression has trailing or concatenated data",
        )?;
        let value: crate::windtun::qualification::unique::Unique = serde_json::from_slice(&raw)
            .map_err(|_| Fault::refused("invalid or duplicate effective source inventory"))?;
        serde_json::from_value(value.0)
            .map_err(|_| Fault::refused("invalid effective source inventory"))
    }

    fn files(files: &[File]) -> Result<(), Fault> {
        let mut total = 0;
        for file in files {
            Wire::require(
                matches!(file.mode.as_str(), "100644" | "100755"),
                "effective source requires regular Git blobs",
            )?;
            Wire::hex(&file.oid, 40)?;
            Wire::hex(&file.sha256, 64)?;
            Wire::require(
                file.size <= Self::TOTAL,
                "effective source blob exceeds bounds",
            )?;
            total += file.size;
            Wire::require(
                total <= Self::TOTAL,
                "effective source total exceeds bounds",
            )?;
            let raw = STANDARD
                .decode(&file.data)
                .map_err(|_| Fault::refused("invalid effective source blob encoding"))?;
            Wire::require(
                raw.len() == file.size
                    && Wire::sha(&raw) == file.sha256
                    && Tree::object("blob", &raw) == file.oid,
                "effective source blob bytes differ",
            )?;
        }
        Ok(())
    }
}
