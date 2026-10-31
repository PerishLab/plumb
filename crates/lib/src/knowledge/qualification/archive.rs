use super::{
    Fault, Read, Result,
    wire::{ARCHIVE, Wire},
};
use flate2::read::GzDecoder;
use object::Object;
use serde_json::{Value, json};
use std::{collections::BTreeMap, io::Read as _};

pub(super) struct Archive;

impl Archive {
    pub fn files(raw: &[u8], names: &[String]) -> Result<BTreeMap<String, Vec<u8>>> {
        let mut decoded = Vec::new();
        GzDecoder::new(raw)
            .take((2 * ARCHIVE + 1) as u64)
            .read_to_end(&mut decoded)
            .map_err(|_| Fault::refused("invalid gzip archive"))?;
        Wire::require(
            decoded.len() <= 2 * ARCHIVE,
            "decoded archive exceeds bounds",
        )?;
        let mut archive = tar::Archive::new(decoded.as_slice());
        let mut files = BTreeMap::new();
        for entry in archive
            .entries()
            .map_err(|_| Fault::refused("invalid tar archive"))?
        {
            let mut entry = entry.map_err(|_| Fault::refused("invalid archive member"))?;
            let path = entry
                .path()
                .map_err(|_| Fault::refused("invalid archive path"))?
                .to_str()
                .ok_or_else(|| Fault::refused("non-UTF8 archive path"))?
                .to_string();
            Wire::require(
                names.contains(&path)
                    && !files.contains_key(&path)
                    && entry.header().entry_type().is_file(),
                "archive has unexpected, repeated or non-file members",
            )?;
            let mut bytes = Vec::new();
            entry
                .read_to_end(&mut bytes)
                .map_err(|_| Fault::refused("truncated archive member"))?;
            Wire::require(bytes.len() <= ARCHIVE, "archive member exceeds bounds")?;
            files.insert(path, bytes);
        }
        Wire::require(
            files.len() == names.len(),
            "archive lacks installed binaries",
        )?;
        Ok(files)
    }

    pub fn identity(raw: &[u8], source: &Value) -> Result<()> {
        let image = object::File::parse(raw).map_err(|_| Fault::refused("invalid executable"))?;
        Wire::require(
            image.format() == object::BinaryFormat::Elf
                && image.architecture() == object::Architecture::X86_64
                && image.is_little_endian(),
            "unsupported executable platform",
        )?;
        let (origin, binding) = crate::identity::inspect(raw).map_err(Fault::refused)?;
        let binding = binding.ok_or_else(|| Fault::refused("executable is unbound"))?;
        let product = Wire::text(&source["repository"])?
            .strip_prefix("PerishLab/")
            .ok_or_else(|| Fault::refused("foreign executable repository"))?;
        let actual = json!({"target":origin.target, "origin":origin.commit, "product":binding.product, "commit":binding.commit, "marker":binding.marker, "digest":binding.digest});
        let expected = json!({"target":"x86_64-unknown-linux-gnu", "origin":"", "product":product, "commit":source["commit"], "marker":source["marker"], "digest":Wire::digest(source)?});
        Wire::matches(
            &actual,
            &expected,
            "executable binding differs from publication",
        )
    }

    pub fn seal<R: Read>(reader: &R, pointer: &Value, authority: &str) -> Result<(Value, Vec<u8>)> {
        Wire::fields(
            pointer,
            &[
                "schema",
                "product",
                "releaseVersion",
                "channel",
                "commit",
                "seal",
                "managers",
            ],
        )?;
        Wire::require(
            pointer["schema"] == 1 && pointer["managers"].is_object(),
            "unknown release pointer",
        )?;
        Wire::hex(&pointer["commit"], 40)?;
        let path = format!(
            "{authority}/v1/releases/{}/{}/seal.json",
            Wire::text(&pointer["channel"])?,
            Wire::text(&pointer["releaseVersion"])?
        );
        Wire::require(
            pointer["seal"]["url"] == path,
            "pointer names a foreign seal",
        )?;
        let raw = Wire::read(reader, &pointer["seal"], authority)?;
        let seal = Wire::parse(&raw, false)?;
        let mut fields = vec![
            "schema",
            "product",
            "releaseVersion",
            "channel",
            "commit",
            "generator",
            "managers",
            "artifacts",
            "provenance",
            "url",
        ];
        if seal.get("guard").is_some() {
            fields.push("guard");
        }
        Wire::fields(&seal, &fields)?;
        Wire::require(
            seal["schema"] == 1
                && seal["url"] == path
                && ["product", "releaseVersion", "channel", "commit"]
                    .iter()
                    .all(|field| seal[*field] == pointer[*field]),
            "seal and pointer identities differ",
        )?;
        Wire::require(
            seal["generator"].is_object()
                && seal["provenance"].is_object()
                && seal["managers"].is_object(),
            "invalid release metadata",
        )?;
        let artifacts = seal["artifacts"]
            .as_object()
            .ok_or_else(|| Fault::refused("missing release artifacts"))?;
        for reference in artifacts.values() {
            Wire::reference(reference, authority)?;
        }
        Ok((seal, raw))
    }
}
