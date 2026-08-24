use super::cloud::Bond;
use super::model::App;
use super::process::Call;
use plumb::rig::Site;
use std::path::Path;
use std::process::Command;

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
    let reached = reach(&app, &site, bond)?;
    println!("  reachable {}", if reached { "yes" } else { "no" });
    Ok("site deploy: ok".into())
}

fn reach(app: &App, site: &Site, bond: Bond) -> Result<bool, String> {
    let stamp = super::artifact::stamp(app)?;
    let root = format!("https://{}/", site.domain);
    let mut live = probe(&root, &stamp, site)?;
    let routes = super::artifact::routes(app)?;
    if live && let Some(route) = super::artifact::deep(&routes) {
        live = probe(&format!("https://{}{route}", site.domain), &stamp, site)?;
    }
    if live {
        return Ok(true);
    }
    if bond == Bond::Unknown {
        return Err(format!(
            "nothing proved {} serves this build: binding is unknown and readback failed",
            site.domain
        ));
    }
    if site.blind {
        println!("  vantage declared blind: public reachability was not proved");
        Ok(false)
    } else {
        Err(format!(
            "{} is attached but did not serve this build; set PLUMB_SITE_BLIND=true only for a known blind vantage",
            site.domain
        ))
    }
}

fn probe(url: &str, stamp: &str, site: &Site) -> Result<bool, String> {
    for turn in 0..site.turns {
        let why = match fetch(url) {
            Ok((200, body)) if body.contains(stamp) => {
                println!("  200 {url} serving {stamp}");
                return Ok(true);
            }
            Ok((200, _)) => "stale build".to_string(),
            Ok((status, _)) => status.to_string(),
            Err(error) => error,
        };
        println!("  retry {url} ({why})");
        if turn + 1 < site.turns && site.delay > 0 {
            std::thread::sleep(std::time::Duration::from_millis(site.delay));
        }
    }
    Ok(false)
}

fn fetch(url: &str) -> Result<(u16, String), String> {
    let output = Command::new("curl")
        .args([
            "--silent",
            "--show-error",
            "--header",
            "Cache-Control: no-cache",
            "--write-out",
            "\n%{http_code}",
            "--url",
            url,
        ])
        .output()
        .map_err(|error| format!("cannot run curl: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "curl failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let (body, status) = text
        .rsplit_once('\n')
        .ok_or_else(|| "curl returned no HTTP status".to_string())?;
    let status = status
        .trim()
        .parse()
        .map_err(|_| format!("invalid HTTP status: {status}"))?;
    Ok((status, body.to_string()))
}
