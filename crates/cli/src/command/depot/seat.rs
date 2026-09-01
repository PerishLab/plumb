use crate::shape::depot::POINTER;
use std::path::Path;

pub enum Held {
    Absent,
    Seat(Box<plumb::depot::Rules>),
    Blind(String),
}

pub use plumb::depot::root;

pub fn held(over: &Path) -> Held {
    if let Some(path) = plumb::config::value("PLUMB_GUARD_CONFIGURATION") {
        return plumb::depot::Rules::guard(Path::new(&path), plumb::version!("PLUMB"))
            .map(|seat| Held::Seat(Box::new(seat)))
            .unwrap_or_else(Held::Blind);
    }
    let base = match root(over) {
        Ok(base) => base,
        Err(error) => return Held::Blind(error),
    };
    if !base.join(plumb::depot::v2::POINTER).is_file() && !base.join(POINTER).is_file() {
        return Held::Absent;
    }
    match plumb::depot::Rules::at(&base, plumb::version!("PLUMB")) {
        Ok(seat) => Held::Seat(Box::new(seat)),
        Err(error) => Held::Blind(error),
    }
}

impl Held {
    pub fn read(&self, path: &str, factory: &'static str) -> Result<String, String> {
        match self {
            Self::Absent => Ok(factory.to_string()),
            Self::Blind(error) => Err(format!("the plumb depot seat is unreadable: {error}")),
            Self::Seat(seat) => seat.read(path),
        }
    }

    pub fn mark(&self) -> Option<String> {
        match self {
            Self::Seat(seat) => Some(seat.mark().to_string()),
            _ => None,
        }
    }
}
