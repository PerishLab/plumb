use crate::command::ship::site::artifact;
use crate::command::ship::site::cloud::Bond;
use crate::command::ship::site::model::App;
use plumb::rig::Site;
use std::process::Command;

pub fn run(app: &App, site: &Site, bond: Bond) -> Result<bool, String> {
    let stamp = artifact::stamp(app)?;
    let root = format!("https://{}/", site.domain);
    let mut live = probe(&root, &stamp, site)?;
    let routes = artifact::routes(app)?;
    if live && let Some(route) = artifact::deep(&routes) {
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
