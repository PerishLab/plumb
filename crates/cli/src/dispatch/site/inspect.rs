use super::model::App;
use std::path::Path;

pub fn run(root: &Path) -> Result<String, String> {
    let app = App::read(root)?;
    let site = super::settings::read()?;
    super::settings::require(&site)?;
    let token = super::cloud::verify(&site)?;
    println!("token: {token}");
    let worker = super::cloud::worker(&site, &app.worker)?;
    println!(
        "worker: {} {}",
        app.worker,
        if worker { "present" } else { "absent" }
    );
    println!("bound: {}", super::cloud::binding(&site));
    Ok("site inspect: ok".into())
}
