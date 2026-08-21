use plumb::forgejo::Client;

pub const SECRETS: [&str; 9] = [
    "RELEASE_ACTIVATE_S3_ACCESS_KEY",
    "RELEASE_ACTIVATE_S3_BUCKET",
    "RELEASE_ACTIVATE_S3_ENDPOINT",
    "RELEASE_ACTIVATE_S3_SECRET_KEY",
    "RELEASE_PUBLISH_S3_ACCESS_KEY",
    "RELEASE_PUBLISH_S3_BUCKET",
    "RELEASE_PUBLISH_S3_ENDPOINT",
    "RELEASE_PUBLISH_S3_SECRET_KEY",
    "RELEASE_REGISTRY_TOKEN",
];

pub fn archive(client: &Client) -> Result<bool, String> {
    let response = client.patch("", serde_json::json!({ "archived": true }))?;
    match response {
        404 => {
            println!("forgejo: repository absent");
            Ok(false)
        }
        200 => {
            println!("forgejo: repository archived");
            Ok(true)
        }
        status => Err(format!("forgejo: archiving repository failed ({status})")),
    }
}

pub fn purge(client: &Client) -> Result<(), String> {
    let mut removed = 0;
    let mut absent = Vec::new();
    for name in SECRETS {
        match client.unset(name)? {
            204 => removed += 1,
            404 => absent.push(name),
            status => return Err(format!("forgejo: deleting secret {name} failed ({status})")),
        }
    }
    println!("forgejo: {removed} release credentials removed");
    if !absent.is_empty() {
        println!(
            "forgejo: {} already absent: {}",
            absent.len(),
            absent.join(" ")
        );
    }
    Ok(())
}

pub fn erase(client: &Client) -> Result<(), String> {
    match client.remove()? {
        204 | 404 => Ok(()),
        status => Err(format!("forgejo: deleting repository failed ({status})")),
    }
}
