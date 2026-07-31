use super::artifact;
use super::cloud::{self, Bond};
use super::model::App;
use plumb::rig::Site;

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
        let why = match cloud::request(url, None) {
            Ok(response) if response.status == 200 && response.body.contains(stamp) => {
                println!("  200 {url} serving {stamp}");
                return Ok(true);
            }
            Ok(response) if response.status == 200 => "stale build".to_string(),
            Ok(response) => response.status.to_string(),
            Err(error) => error,
        };
        println!("  retry {url} ({why})");
        if turn + 1 < site.turns && site.delay > 0 {
            std::thread::sleep(std::time::Duration::from_millis(site.delay));
        }
    }
    Ok(false)
}
