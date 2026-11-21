use super::{Declaration, ENDPOINT, Settings};
use locus::reporter::Spec;
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Default, Deserialize)]
pub struct Configuration {
    #[serde(default)]
    enabled: bool,
    endpoint: Option<String>,
    #[serde(rename = "key_file")]
    writer: Option<PathBuf>,
    sender: Option<PathBuf>,
}

impl Configuration {
    pub fn new(settings: Settings, writer: Option<PathBuf>) -> Self {
        Self {
            enabled: settings.enabled,
            endpoint: settings.endpoint,
            writer,
            sender: None,
        }
    }

    pub fn sender(mut self, executable: PathBuf) -> Self {
        self.sender = Some(executable);
        self
    }

    pub(super) fn fallback(
        mut self,
        fallback: impl FnOnce() -> (Option<String>, Option<PathBuf>, Option<PathBuf>),
    ) -> Self {
        if !self.enabled {
            return self;
        }
        if let (None, None, None) = (&self.endpoint, &self.writer, &self.sender) {
            (self.endpoint, self.writer, self.sender) = fallback();
        }
        self
    }

    pub(super) fn reporter(self, declaration: &Declaration<'_>) -> Result<Option<Spec>, String> {
        if !self.enabled {
            return Ok(None);
        }
        let Some(writer) = self.writer else {
            if self.sender.is_some() {
                return Err(
                    "observation sender requires an HTTPS endpoint and writer key_file".into(),
                );
            }
            if self
                .endpoint
                .as_deref()
                .is_some_and(|v| v.starts_with("https://"))
            {
                return Err("HTTPS observation needs a matching writer key_file".into());
            }
            return Ok(Some(Spec::api(
                self.endpoint.unwrap_or_else(|| ENDPOINT.into()),
                &declaration.buffer,
            )));
        };
        let endpoint = self.endpoint.filter(|v| v.starts_with("https://")).ok_or(
            "observation key_file requires an explicit HTTPS endpoint from the same configuration",
        )?;
        if !writer.is_absolute() || writer.to_str().is_none() {
            return Err("observation key_file must be an absolute UTF-8 file path".into());
        }
        if !declaration.buffer.is_absolute()
            || declaration.buffer.file_name().is_none()
            || declaration.buffer.to_str().is_none()
        {
            return Err(
                "HTTPS observation requires an absolute UTF-8 product buffer directory".into(),
            );
        }
        let producer = Path::new(declaration.producer);
        if producer.file_name() != Some(producer.as_os_str()) {
            return Err(
                "HTTPS observation requires a producer name with one path component".into(),
            );
        }
        let buffer = declaration
            .buffer
            .with_added_extension("https")
            .join(producer);
        let spec = Spec::https(endpoint, writer, buffer);
        match self.sender {
            Some(path) if !path.is_absolute() || path.to_str().is_none() => {
                Err("observation sender must be an absolute UTF-8 executable path".into())
            }
            Some(path) => Ok(Some(spec.sender(path))),
            None => Ok(Some(spec)),
        }
    }
}
