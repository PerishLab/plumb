use super::{Binding, Origin, codec};
use object::{Object, ObjectKind, ObjectSection};
use std::ops::Range;

pub fn inspect(bytes: &[u8]) -> Result<(Origin, Option<Binding>), String> {
    codec::Codec(&bytes[locate(bytes)?]).decode()
}

pub fn bind(bytes: &[u8], binding: &Binding) -> Result<Vec<u8>, String> {
    let range = locate(bytes)?;
    let updated = codec::Codec(&bytes[range.clone()]).encode(binding)?;
    let mut result = bytes.to_vec();
    result[range].copy_from_slice(&updated);
    if inspect(&result)?.1.as_ref() != Some(binding) {
        return Err("bound executable identity did not read back".into());
    }
    Ok(result)
}

fn locate(bytes: &[u8]) -> Result<Range<usize>, String> {
    let file =
        object::File::parse(bytes).map_err(|error| format!("cannot parse executable: {error}"))?;
    if !matches!(file.kind(), ObjectKind::Executable | ObjectKind::Dynamic) {
        return Err("identity binding requires a linked executable".into());
    }
    let signed = match &file {
        object::File::Pe32(held) => held
            .data_directory(object::pe::IMAGE_DIRECTORY_ENTRY_SECURITY)
            .is_some(),
        object::File::Pe64(held) => held
            .data_directory(object::pe::IMAGE_DIRECTORY_ENTRY_SECURITY)
            .is_some(),
        _ => false,
    };
    if signed {
        return Err("identity binding refuses an Authenticode-signed input".into());
    }
    let mut found = None;
    for section in file.sections() {
        let name = section.name().map_err(|error| error.to_string())?;
        if name != ".releaseid" && name != "__releaseid" {
            continue;
        }
        if found.is_some() {
            return Err("executable has multiple identity regions".into());
        }
        if name == "__releaseid"
            && section.segment_name().map_err(|error| error.to_string())? != Some("__DATA")
        {
            return Err("Mach-O identity must reside in its reserved data segment".into());
        }
        let (offset, size) = section
            .file_range()
            .ok_or("identity region has no file content")?;
        let start = usize::try_from(offset).map_err(|_| "identity offset exceeds file bounds")?;
        let size = usize::try_from(size).map_err(|_| "identity size exceeds file bounds")?;
        let end = start.checked_add(size).ok_or("identity range overflow")?;
        if end > bytes.len() || size != super::SIZE {
            return Err("identity region has invalid bounds".into());
        }
        if section.relocations().next().is_some() {
            return Err("identity region must not carry relocations".into());
        }
        found = Some(start..end);
    }
    found.ok_or_else(|| "executable has no release identity region".into())
}
