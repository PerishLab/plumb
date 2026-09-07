pub use command::Deed;
use plumb::rig::Rig;
use plumb::skill::{Kit, command};
use std::path::PathBuf;

pub fn run(deed: Deed) -> i32 {
    let rig = match Rig::resolve(None) {
        Ok(rig) => rig,
        Err(error) => return sour(&error.to_string()),
    };
    if rig.home.is_empty() {
        return sour("no data home; set PLUMB_HOME");
    }
    let kit = Kit {
        name: "plumb".to_string(),
        home: plumb::config::home().unwrap_or_else(|| PathBuf::from(".")),
        state: PathBuf::from(&rig.home).join("state").join("skills.json"),
        url: rig.releases.clone(),
    };
    let depot = kit.depot(&rig.rules.source, "plumb", plumb::version!("PLUMB"));
    command::Command("plumb").run(&depot, deed)
}

fn sour(note: &str) -> i32 {
    eprintln!("plumb skill: {note}");
    1
}
