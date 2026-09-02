use sha2::Digest as _;
use std::{
    collections::BTreeMap, fs, io::Write, path::Path, process::Command, thread, time::Duration,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct View {
    pub access: String,
    pub bucket: String,
    pub endpoint: String,
}

pub struct Escrow {
    pub access: String,
    pub secret: String,
    pub bucket: String,
    pub endpoint: String,
}

pub struct Seat<'a>(&'a Path);

impl<'a> Seat<'a> {
    pub fn new(path: &'a Path) -> Self {
        Self(path)
    }

    pub fn load(&self) -> Result<Option<Escrow>, String> {
        let path = self.0;
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(format!("cannot read {}: {error}", path.display())),
        };
        self.mode()?;
        let mut fields = BTreeMap::new();
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            let (name, value) = line
                .split_once('=')
                .ok_or_else(|| format!("invalid release escrow {}", path.display()))?;
            if !super::SECRETS.contains(&name)
                || value.is_empty()
                || fields.insert(name, value).is_some()
            {
                return Err(format!("invalid release escrow {}", path.display()));
            }
        }
        if fields.len() != super::SECRETS.len() {
            return Err(format!("incomplete release escrow {}", path.display()));
        }
        Ok(Some(Escrow {
            access: fields[super::SECRETS[0]].to_string(),
            secret: fields[super::SECRETS[1]].to_string(),
            bucket: fields[super::SECRETS[2]].to_string(),
            endpoint: fields[super::SECRETS[3]].to_string(),
        }))
    }

    pub fn write(&self, held: &Escrow) -> Result<(), String> {
        self.publish(held, false)
    }

    pub fn replace(&self, held: &Escrow) -> Result<(), String> {
        self.publish(held, true)
    }

    fn publish(&self, held: &Escrow, replace: bool) -> Result<(), String> {
        let path = self.0;
        let parent = path
            .parent()
            .ok_or_else(|| "release escrow has no parent".to_string())?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
        let mut draft = tempfile::NamedTempFile::new_in(parent)
            .map_err(|error| format!("cannot create release escrow draft: {error}"))?;
        writeln!(
            draft,
            "{}={}\n{}={}\n{}={}\n{}={}",
            super::SECRETS[0],
            held.access,
            super::SECRETS[1],
            held.secret,
            super::SECRETS[2],
            held.bucket,
            super::SECRETS[3],
            held.endpoint,
        )
        .map_err(|error| format!("cannot write release escrow draft: {error}"))?;
        draft
            .as_file()
            .sync_all()
            .map_err(|error| format!("cannot sync release escrow draft: {error}"))?;
        protect(draft.as_file())?;
        if replace {
            draft
                .persist(path)
                .map_err(|error| format!("cannot replace {}: {}", path.display(), error.error))?;
        } else {
            draft
                .persist_noclobber(path)
                .map_err(|error| format!("cannot publish {}: {error}", path.display()))?;
        }
        Ok(())
    }

    #[cfg(unix)]
    fn mode(&self) -> Result<(), String> {
        use std::os::unix::fs::PermissionsExt;
        let path = self.0;
        let mode = fs::metadata(path)
            .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?
            .permissions()
            .mode()
            & 0o777;
        if mode == 0o600 {
            Ok(())
        } else {
            Err(format!("{} must be mode 600", path.display()))
        }
    }

    #[cfg(not(unix))]
    fn mode(&self) -> Result<(), String> {
        fs::metadata(self.0)
            .map(|_| ())
            .map_err(|error| format!("cannot inspect {}: {error}", self.0.display()))
    }
}

impl Escrow {
    pub fn view(&self) -> View {
        View {
            access: self.access.clone(),
            bucket: self.bucket.clone(),
            endpoint: self.endpoint.clone(),
        }
    }

    pub fn exact(&self, bucket: &str, account: &str) -> Result<(), String> {
        let endpoint = format!("https://{account}.r2.cloudflarestorage.com");
        if self.bucket == bucket && self.endpoint == endpoint {
            Ok(())
        } else {
            Err("release escrow target disagrees with the derived authority".into())
        }
    }

    pub fn verify(&self) -> Result<(), String> {
        let mut failure = String::new();
        for turn in 0..8 {
            let output = Command::new("aws")
                .args([
                    "--endpoint-url",
                    &self.endpoint,
                    "s3api",
                    "list-objects-v2",
                    "--bucket",
                    &self.bucket,
                    "--max-keys",
                    "1",
                ])
                .env("AWS_ACCESS_KEY_ID", &self.access)
                .env("AWS_SECRET_ACCESS_KEY", &self.secret)
                .env("AWS_DEFAULT_REGION", "auto")
                .env("AWS_EC2_METADATA_DISABLED", "true")
                .output()
                .map_err(|error| format!("cannot execute aws S3 probe: {error}"))?;
            if output.status.success() {
                return Ok(());
            }
            failure = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if turn < 7 {
                thread::sleep(Duration::from_secs(2));
            }
        }
        Err(format!("release capability failed S3 readback: {failure}"))
    }

    pub fn minted(access: String, value: &str, bucket: String, account: &str) -> Self {
        Self {
            access,
            secret: sha256(value),
            bucket,
            endpoint: format!("https://{account}.r2.cloudflarestorage.com"),
        }
    }
}

fn sha256(value: &str) -> String {
    sha2::Sha256::digest(value.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(super) fn fingerprint(endpoint: &str) -> String {
    sha256(endpoint.trim_end_matches('/'))
}

#[cfg(unix)]
fn protect(file: &fs::File) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    file.set_permissions(fs::Permissions::from_mode(0o600))
        .map_err(|error| format!("cannot protect release escrow draft: {error}"))
}

#[cfg(not(unix))]
fn protect(_: &fs::File) -> Result<(), String> {
    Ok(())
}
