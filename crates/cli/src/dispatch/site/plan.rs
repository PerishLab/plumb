use super::model::App;
use super::process::Call;
use std::path::Path;

pub fn run(root: &Path) -> Result<String, String> {
    let app = App::read(root)?;
    let site = super::settings::read()?;
    let domain = if site.domain.is_empty() {
        "<PLUMB_SITE_DOMAIN>"
    } else {
        &site.domain
    };
    println!("site plan");
    println!("  app       {}", app.seat.display());
    println!("  build     pnpm --filter {} build", app.package);
    println!("  deploy    pnpm exec wrangler deploy --domain {domain}");
    println!("  worker    {}", app.worker);
    println!("  verify    https://{domain}/");
    if let Some(route) = super::artifact::deep(&super::artifact::routes(&app)?) {
        println!("  verify    https://{domain}{route}");
    }
    if !app.index().is_file() {
        return Ok(format!(
            "wrangler dry run skipped: {} does not exist",
            app.index().display()
        ));
    }
    let mut args = vec!["exec", "wrangler", "deploy", "--dry-run"];
    if !site.domain.is_empty() {
        args.extend(["--domain", site.domain.as_str()]);
    }
    super::process::run(Call {
        bin: "pnpm",
        args: &args,
        cwd: &app.seat,
        env: &[],
    })?;
    Ok("site plan: ok".into())
}
