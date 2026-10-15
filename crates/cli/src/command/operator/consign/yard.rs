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
    plumb::config::value(&format!("{SEAT}_{name}")).ok_or_else(|| unset(&[name]))
}

fn secret() -> Result<String, String> {
    if let Some(held) = plumb::config::value(&format!("{SEAT}_SECRET_ACCESS_KEY")) {
        return Ok(held);
    }
    let path = plumb::config::value(&format!("{SEAT}_SECRET_ACCESS_KEY_FILE"))
        .ok_or_else(|| unset(&["SECRET_ACCESS_KEY", "SECRET_ACCESS_KEY_FILE"]))?;
    std::fs::read_to_string(&path)
        .map(|held| held.trim().to_string())
        .map_err(|error| format!("cannot read the yard secret at {path}: {error}"))
}

fn unset(names: &[&str]) -> String {
    let names = names
        .iter()
        .map(|name| format!("{SEAT}_{name}"))
        .collect::<Vec<_>>()
        .join(" or ");
    format!(
        "{names} is unset; consign runs where the yard credential issued to this holder is configured"
    )
}

#[cfg(test)]
mod tests {
    use super::unset;

    #[test]
    fn unnamed() {
        let field = unset(&["BUCKET"]);
        assert!(field.starts_with("PLUMB_YARD_BUCKET is unset;"), "{field}");
        assert!(!field.contains("runseal"), "{field}");
        let secret = unset(&["SECRET_ACCESS_KEY", "SECRET_ACCESS_KEY_FILE"]);
        assert!(
            secret.starts_with(
                "PLUMB_YARD_SECRET_ACCESS_KEY or PLUMB_YARD_SECRET_ACCESS_KEY_FILE is unset;"
            ),
            "{secret}"
        );
    }
}
