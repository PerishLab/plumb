use super::{
    Fault, Result,
    wire::{LIMIT, Wire},
};
use std::io::Read;

pub(super) struct Registry<'a>(pub &'a ureq::Agent);

impl Registry<'_> {
    pub fn inspect(&self, reference: &str) -> Result<String> {
        let path = reference.strip_prefix("ghcr.io/").ok_or_else(|| {
            Fault::unknown("runtime registry is unsupported; no qualification inferred")
        })?;
        let (repository, marker) = if let Some((name, digest)) = path.split_once('@') {
            (name, digest)
        } else {
            path.rsplit_once(':')
                .ok_or_else(|| Fault::refused("runtime image has no resolved marker"))?
        };
        Wire::require(
            repository.split('/').all(|part| {
                !matches!(part, "" | "." | "..")
                    && part.bytes().all(|v| {
                        v.is_ascii_lowercase() || v.is_ascii_digit() || b"_.-".contains(&v)
                    })
            }),
            "runtime repository is invalid",
        )?;
        Wire::require(
            !marker.is_empty()
                && marker
                    .bytes()
                    .all(|v| v.is_ascii_alphanumeric() || b"._-:".contains(&v)),
            "runtime marker is invalid",
        )?;
        let token = self.token(repository)?;
        let url = format!("https://ghcr.io/v2/{repository}/manifests/{marker}");
        let started = std::time::Instant::now();
        let response = match self.0.get(&url).header("Authorization", &format!("Bearer {token}")).header("Accept", "application/vnd.oci.image.index.v1+json,application/vnd.oci.image.manifest.v1+json,application/vnd.docker.distribution.manifest.list.v2+json,application/vnd.docker.distribution.manifest.v2+json").call() {
            Ok(response) => response,
            Err(error) => {
                let code = match error {
                    ureq::Error::StatusCode(code) => Some(code),
                    _ => None,
                };
                crate::runtime::exchange::https("registry.manifest", "error", code, started.elapsed());
                return Err(Fault::unknown("runtime manifest is not anonymously readable"));
            }
        };
        let status = response.status().as_u16();
        let header = response
            .headers()
            .get("docker-content-digest")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        let bytes = match self.body(response) {
            Ok(bytes) => {
                crate::runtime::exchange::https(
                    "registry.manifest",
                    "ok",
                    Some(status),
                    started.elapsed(),
                );
                bytes
            }
            Err(error) => {
                crate::runtime::exchange::https(
                    "registry.manifest",
                    "error",
                    Some(status),
                    started.elapsed(),
                );
                return Err(error);
            }
        };
        let value = Wire::parse(&bytes, false)?;
        Wire::require(
            value["schemaVersion"] == 2
                && (value["manifests"].is_array() || value["layers"].is_array()),
            "unsupported runtime manifest",
        )?;
        let digest = format!("sha256:{}", Wire::sha(&bytes));
        Wire::require(
            header.as_ref().is_none_or(|value| *value == digest),
            "runtime manifest digest header differs",
        )?;
        if reference.contains('@') {
            Wire::require(
                marker == digest,
                "runtime manifest differs from immutable reference",
            )?;
        }
        Ok(digest)
    }

    fn token(&self, repository: &str) -> Result<String> {
        let started = std::time::Instant::now();
        let response = match self
            .0
            .get(&format!(
                "https://ghcr.io/token?service=ghcr.io&scope=repository:{repository}:pull"
            ))
            .call()
        {
            Ok(response) => response,
            Err(error) => {
                let code = match error {
                    ureq::Error::StatusCode(code) => Some(code),
                    _ => None,
                };
                crate::runtime::exchange::https("registry.token", "error", code, started.elapsed());
                return Err(Fault::unknown(
                    "anonymous runtime registry admission failed",
                ));
            }
        };
        let status = response.status().as_u16();
        let bytes = match self.body(response) {
            Ok(bytes) => {
                crate::runtime::exchange::https(
                    "registry.token",
                    "ok",
                    Some(status),
                    started.elapsed(),
                );
                bytes
            }
            Err(error) => {
                crate::runtime::exchange::https(
                    "registry.token",
                    "error",
                    Some(status),
                    started.elapsed(),
                );
                return Err(error);
            }
        };
        let value = Wire::parse(&bytes, false)?;
        Ok(Wire::text(
            value
                .get("token")
                .or_else(|| value.get("access_token"))
                .unwrap_or(&serde_json::Value::Null),
        )?
        .into())
    }

    fn body(&self, response: ureq::http::Response<ureq::Body>) -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        response
            .into_body()
            .as_reader()
            .take(LIMIT as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| Fault::unknown("runtime registry read failed"))?;
        Wire::require(
            bytes.len() <= LIMIT,
            "runtime registry material exceeds bounds",
        )?;
        Ok(bytes)
    }
}
