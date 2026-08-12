use super::cloud::Bond;
use super::model::App;
use super::process::Call;
use std::path::Path;

pub fn run(root: &Path) -> Result<String, String> {
    let app = App::read(root)?;
    let site = super::settings::read()?;
    super::settings::require(&site)?;
    let marks = super::artifact::marks(&app)?;
    println!("build");
    super::process::run(Call {
        bin: "pnpm",
        args: &["--filter", &app.package, "build"],
        cwd: &app.root,
        env: &[
            ("BUILD_COMMIT", &marks.commit),
            ("BUILD_VERSION", &marks.version),
        ],
    })?;
    if !app.index().is_file() {
        return Err(format!("build produced no {}", app.index().display()));
    }
    println!("deploy");
    super::process::run(Call {
        bin: "pnpm",
        args: &["exec", "wrangler", "deploy", "--domain", &site.domain],
        cwd: &app.seat,
        env: &[
            ("CLOUDFLARE_ACCOUNT_ID", &site.account),
            ("CLOUDFLARE_API_TOKEN", &site.token),
        ],
    })?;
    println!("verify");
    println!("  deployed  yes");
    let bond = super::cloud::Vantage::new(&site).binding();
    println!("  bound     {bond}");
    match bond {
        Bond::No => return Err(format!("{} is not attached to the worker", site.domain)),
        Bond::Unknown => {
            println!("  binding could not be read; reachability must carry the proof")
        }
        Bond::Yes => {}
    }
    let reached = super::reach::run(&app, &site, bond)?;
    println!("  reachable {}", if reached { "yes" } else { "no" });
    Ok("site deploy: ok".into())
}
