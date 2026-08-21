use super::model::App;
use std::path::Path;

pub fn run(root: &Path) -> Result<String, String> {
    let app = App::read(root)?;
    let site = super::settings::read()?;
    super::settings::require(&site)?;
    let vantage = super::cloud::Vantage::new(&site);
    let token = vantage.verify()?;
    println!("token: {token}");
    let worker = vantage.worker(&app.worker)?;
    println!(
        "worker: {} {}",
        app.worker,
        if worker { "present" } else { "absent" }
    );
    println!("bound: {}", vantage.binding());
    Ok("site inspect: ok".into())
}
