use super::{Manifest, Plan, Receipt, SCHEMA};
use crate::command::cache::transport::{MARKER, stamp::Stamp, target::NAMES};
use std::collections::BTreeMap;
use std::fs::{File, Metadata};
use std::path::{Path, PathBuf};

struct Writer {
    tar: tar::Builder<zstd::Encoder<'static, tempfile::NamedTempFile>>,
    manifest: Manifest,
}

pub(super) fn write(path: &Path, plan: Plan) -> Result<Receipt, String> {
    let parent = path.parent().ok_or("compilation parcel has no parent")?;
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    if let Ok(metadata) = std::fs::symlink_metadata(path)
        && (!metadata.is_file() || metadata.file_type().is_symlink())
    {
        return Err("compilation parcel is not a regular owned file".into());
    }
    let file = tempfile::NamedTempFile::new_in(parent).map_err(|error| error.to_string())?;
    let mut writer = Writer {
        tar: tar::Builder::new(zstd::Encoder::new(file, 1).map_err(|error| error.to_string())?),
        manifest: Manifest {
            schema: SCHEMA.into(),
            identity: plan.identity,
            cargo: plan.cargo.is_some(),
            compiler: false,
            times: BTreeMap::new(),
        },
    };
    if let Some(root) = plan.cargo {
        for name in NAMES.into_iter().chain([MARKER]) {
            let path = root.join(name);
            if path.exists() {
                writer.append(&path, &Path::new("cargo").join(name))?;
            }
        }
    }
    if let Some(root) = plan.compiler {
        let before = writer.manifest.times.len();
        if root.exists() {
            writer.append(&root, Path::new("compiler"))?;
        }
        writer.manifest.compiler = writer.manifest.times.len() != before;
    }
    if !writer.manifest.cargo && !writer.manifest.compiler {
        return Ok(Receipt::default());
    }
    let receipt = Receipt {
        cargo: writer.manifest.cargo,
        compiler: writer.manifest.compiler,
    };
    writer.finish(path)?;
    Ok(receipt)
}

impl Writer {
    fn append(&mut self, source: &Path, name: &Path) -> Result<(), String> {
        let metadata = std::fs::symlink_metadata(source).map_err(|error| error.to_string())?;
        if metadata.is_dir() {
            self.directory(source, name, &metadata)
        } else if metadata.is_file() {
            self.file(source, name)
        } else {
            Err("compilation parcel refuses links and special files".into())
        }
    }

    fn directory(&mut self, source: &Path, name: &Path, metadata: &Metadata) -> Result<(), String> {
        let mut header = header(metadata)?;
        header.set_entry_type(tar::EntryType::Directory);
        header.set_size(0);
        header.set_cksum();
        self.tar
            .append_data(&mut header, name, std::io::empty())
            .map_err(|error| error.to_string())?;
        let mut paths = std::fs::read_dir(source)
            .map_err(|error| error.to_string())?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<PathBuf>, _>>()
            .map_err(|error| error.to_string())?;
        paths.sort();
        for path in paths {
            let leaf = path
                .file_name()
                .ok_or("compilation parcel entry has no name")?;
            self.append(&path, &name.join(leaf))?;
        }
        Ok(())
    }

    fn file(&mut self, source: &Path, name: &Path) -> Result<(), String> {
        let file = match File::open(source) {
            Ok(file) => file,
            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound && name.starts_with("compiler") =>
            {
                return Ok(());
            }
            Err(error) => return Err(error.to_string()),
        };
        let metadata = file.metadata().map_err(|error| error.to_string())?;
        let stamp = Stamp::at(metadata.modified().map_err(|error| error.to_string())?)?;
        let name = name
            .to_str()
            .ok_or("compilation parcel path is not UTF-8")?
            .replace('\\', "/");
        let mut header = header(&metadata)?;
        header.set_cksum();
        self.tar
            .append_data(&mut header, &name, file)
            .map_err(|error| error.to_string())?;
        self.manifest.times.insert(name, stamp);
        Ok(())
    }

    fn finish(mut self, path: &Path) -> Result<(), String> {
        let bytes = serde_json::to_vec(&self.manifest).map_err(|error| error.to_string())?;
        if bytes.len() > 8 * 1024 * 1024 {
            return Err("compilation parcel manifest exceeds 8 MiB".into());
        }
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        self.tar
            .append_data(&mut header, "manifest.json", bytes.as_slice())
            .map_err(|error| error.to_string())?;
        let encoder = self.tar.into_inner().map_err(|error| error.to_string())?;
        let file = encoder.finish().map_err(|error| error.to_string())?;
        file.as_file()
            .sync_all()
            .map_err(|error| error.to_string())?;
        file.persist(path)
            .map_err(|error| error.error.to_string())?;
        Ok(())
    }
}

fn header(metadata: &Metadata) -> Result<tar::Header, String> {
    let mut header = tar::Header::new_gnu();
    header.set_metadata(metadata);
    #[cfg(unix)]
    header.set_mode(super::super::transport::inputs::mode(metadata));
    #[cfg(not(unix))]
    header.set_mode(if metadata.is_dir() {
        0o755
    } else if metadata.permissions().readonly() {
        0o444
    } else {
        0o644
    });
    header.set_uid(0);
    header.set_gid(0);
    Ok(header)
}
