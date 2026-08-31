use plumb::rig::Site;
use std::path::Path;

pub fn prove(url: &str, site: &Site) -> Result<(), String> {
    let mut why = String::new();
    for turn in 0..site.turns.max(1) {
        why = match answered(url) {
            Ok(true) => {
                println!("  200 {url}");
                return Ok(());
            }
            Ok(false) => format!("{url} did not answer 200"),
            Err(error) => error,
        };
        println!("  retry {url} ({why})");
        if turn + 1 < site.turns && site.delay > 0 {
            std::thread::sleep(std::time::Duration::from_millis(site.delay));
        }
    }
    Err(format!("{url} never answered: {why}"))
}

fn answered(url: &str) -> Result<bool, String> {
    let code = super::process::text(
        "curl",
        &[
            "--silent",
            "--show-error",
            "--location",
            "--connect-timeout",
            "10",
            "--max-time",
            "30",
            "--output",
            "/dev/null",
            "--write-out",
            "%{http_code}",
            url,
        ],
        Path::new("."),
    )?;
    Ok(code == "200")
}
