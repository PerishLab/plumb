use super::origin::Archive;
use base64::Engine;
use sha2::{Digest, Sha512};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;

pub(super) fn identity(integrity: &str) -> Result<String, String> {
    let body = integrity
        .strip_prefix("sha512-")
        .ok_or("only SHA512 npm origin integrity is qualified")?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(body)
        .map_err(|error| error.to_string())?;
    if bytes.len() != 64 {
        return Err("npm origin integrity has an invalid SHA512 length".into());
    }
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

pub(super) fn payload(path: &Path, integrity: &str) -> Result<Vec<u8>, String> {
    let metadata = std::fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if !metadata.is_file() || metadata.len() > 32 * 1024 * 1024 {
        return Err("npm origin archive must be a bounded regular file".into());
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|error| error.to_string())?
        .take(32 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() > 32 * 1024 * 1024
        || format!("{:x}", Sha512::digest(&bytes)) != identity(integrity)?
    {
        return Err("npm origin archive differs from captured registry integrity".into());
    }
    Ok(bytes)
}

impl Archive {
    pub(crate) fn decode(bytes: &[u8]) -> Result<Self, String> {
        let decoder = flate2::read::MultiGzDecoder::new(bytes);
        let mut material = Vec::new();
        decoder
            .take(128 * 1024 * 1024 + 1)
            .read_to_end(&mut material)
            .map_err(|error| error.to_string())?;
        if material.len() > 128 * 1024 * 1024 {
            return Err("npm origin archive exceeded its decompression bound".into());
        }
        let mut archive = tar::Archive::new(std::io::Cursor::new(&material));
        let mut files = BTreeMap::new();
        let mut total = 0u64;
        for entry in archive.entries().map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            let path = entry
                .path()
                .map_err(|error| error.to_string())?
                .into_owned();
            if entry.header().entry_type().is_dir() {
                continue;
            }
            let name = path
                .to_str()
                .and_then(|name| name.strip_prefix("package/"))
                .ok_or("npm origin entry is outside package/")?;
            let kind = entry.header().entry_type().is_file();
            if !kind || name.is_empty() || name.contains('\\') {
                return Err("npm origin archive contains an unsupported entry".into());
            }
            if name
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
            {
                return Err("npm origin archive contains an unsupported entry".into());
            }
            let length = entry.size();
            total = total
                .checked_add(length)
                .ok_or("npm origin archive size overflow")?;
            if total > 128 * 1024 * 1024 || length > 16 * 1024 * 1024 || files.len() >= 4096 {
                return Err("npm origin archive exceeded its material bound".into());
            }
            let mut bytes = Vec::new();
            entry
                .take(length + 1)
                .read_to_end(&mut bytes)
                .map_err(|error| error.to_string())?;
            if bytes.len() as u64 != length || files.insert(name.to_string(), bytes).is_some() {
                return Err("npm origin archive contains a duplicate or incomplete file".into());
            }
        }
        let consumed = archive.into_inner().position() as usize;
        if material[consumed..].iter().any(|byte| *byte != 0) {
            return Err("npm origin archive contains trailing material".into());
        }
        Ok(Self { files })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn archive(entries: &[(&str, &[u8], tar::EntryType)]) -> Vec<u8> {
        let writer = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        let mut archive = tar::Builder::new(writer);
        for (name, body, kind) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_mode(0o644);
            header.set_size(body.len() as u64);
            header.set_entry_type(*kind);
            let path = name.as_bytes();
            header.as_mut_bytes()[..path.len()].copy_from_slice(path);
            header.set_cksum();
            archive.append(&header, *body).unwrap();
        }
        archive.into_inner().unwrap().finish().unwrap()
    }

    #[test]
    fn paths() {
        let plain = tar::EntryType::Regular;
        let valid = archive(&[("package/media/icon.svg", b"pixels", plain)]);
        assert_eq!(
            Archive::decode(&valid).unwrap().files["media/icon.svg"],
            b"pixels"
        );
        for name in [
            "outside/icon.svg",
            "package/../outside.svg",
            "package/media/../../outside.svg",
            "package/media\\icon.svg",
        ] {
            assert!(
                Archive::decode(&archive(&[(name, b"pixels", plain)])).is_err(),
                "{name}"
            );
        }
        let duplicate = archive(&[
            ("package/image.svg", b"one", plain),
            ("package/image.svg", b"two", plain),
        ]);
        assert!(Archive::decode(&duplicate).is_err());
        let mut joined = valid.clone();
        joined.extend_from_slice(&valid);
        assert!(Archive::decode(&joined).is_err());
        assert!(
            Archive::decode(&archive(&[(
                "package/image.svg",
                b"",
                tar::EntryType::Symlink
            )]))
            .is_err()
        );
    }

    #[test]
    fn integrity() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("archive.tgz");
        let bytes = archive(&[("package/image.svg", b"pixels", tar::EntryType::Regular)]);
        std::fs::write(&path, &bytes).unwrap();
        let integrity = format!(
            "sha512-{}",
            base64::engine::general_purpose::STANDARD.encode(Sha512::digest(&bytes))
        );
        assert_eq!(payload(&path, &integrity).unwrap(), bytes);
        std::fs::write(&path, b"changed archive").unwrap();
        assert!(payload(&path, &integrity).is_err());
        assert!(identity("sha256-YQ==").is_err());
        assert!(identity("sha512-YQ==").is_err());
    }
}
