use crate::command::authority::cloudflare::{Factory, Grant, Resource};
use plumb::rig::Authority;
use sha2::{Digest, Sha256};

pub fn project<T>(
    mut direct: Authority,
    bucket: &str,
    marker: &str,
    action: impl FnOnce(&Authority) -> Result<T, String>,
) -> Result<T, String> {
    if direct != Authority::default() {
        direct.load()?;
        if direct.bucket != bucket {
            return Err(format!(
                "activation authority targets {}, not {bucket}",
                direct.bucket
            ));
        }
        if direct.access.is_empty()
            || direct.secret.is_empty()
            || !direct.endpoint.starts_with("https://")
        {
            return Err("incomplete S3 activation authority; factory fallback refused".into());
        }
        return action(&direct);
    }
    let factory = Factory::resolve()?;
    factory.verify()?;
    let permission = factory.permission(
        "Workers R2 Storage Bucket Item Write",
        "com.cloudflare.edge.r2.bucket",
    )?;
    let invocation = tempfile::Builder::new()
        .prefix("plumb-depot-")
        .tempdir()
        .map_err(|error| format!("cannot identify depot invocation: {error}"))?;
    let nonce = invocation.path().file_name().unwrap().to_string_lossy();
    let grant = Grant {
        name: format!("depot:{marker}:{nonce}"),
        permission,
        resource: Resource::Set {
            account: factory.id().to_string(),
            buckets: vec![bucket.to_string()],
        },
        expires: crate::command::clock::ahead(15)?,
    };
    factory.session(&grant, |minted| {
        let authority = Authority {
            access: minted.id.clone(),
            secret: format!("{:x}", Sha256::digest(minted.value().as_bytes())),
            bucket: bucket.to_string(),
            endpoint: format!("https://{}.r2.cloudflarestorage.com", factory.id()),
            ..Authority::default()
        };
        action(&authority)
    })
}
