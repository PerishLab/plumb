use plumb::bucket::{Condition, Control, Outcome, Policy};

const SEAT: &str = "PLUMB_YARD";

pub fn write(key: &str, body: &[u8]) -> Result<&'static str, String> {
    let control = Control::new(
        field("ACCESS_KEY_ID")?,
        secret()?,
        field("BUCKET")?,
        field("ENDPOINT")?,
    )?;
    let policy = Policy {
        media: "application/json",
        cache: "no-store",
    };
    match control.write(key, body, policy, Condition::Absent)? {
        Outcome::Held(()) => Ok("consigned"),
        Outcome::Stale => Ok("already in the yard"),
        Outcome::Missing => Err(format!("the yard answered missing for {key}")),
    }
}

fn field(name: &str) -> Result<String, String> {
    plumb::config::value(&format!("{SEAT}_{name}")).ok_or_else(|| {
        format!("{SEAT}_{name} is unset; consign runs under the profile that holds the yard seat, runseal :liberte")
    })
}

fn secret() -> Result<String, String> {
    if let Some(held) = plumb::config::value(&format!("{SEAT}_SECRET_ACCESS_KEY")) {
        return Ok(held);
    }
    let path = field("SECRET_ACCESS_KEY_FILE")?;
    std::fs::read_to_string(&path)
        .map(|held| held.trim().to_string())
        .map_err(|error| format!("cannot read the yard secret at {path}: {error}"))
}
