use serde::Serialize;
use std::path::Path;

pub(super) const SPAN: u64 = 2;
const RESERVE: u64 = 30;

#[derive(Clone, Copy, Serialize)]
pub(super) struct Disk {
    pub available: u64,
    pub total: u64,
    pub reserve: u64,
    pub pressure: bool,
}

impl Disk {
    pub fn read(path: &Path) -> Result<Self, String> {
        let reserve = match plumb::config::value("PLUMB_GUARD_CACHE_RESERVE") {
            None => RESERVE,
            Some(value) => value
                .parse()
                .ok()
                .filter(|percent| *percent <= 100)
                .ok_or_else(|| {
                    format!(
                        "PLUMB_GUARD_CACHE_RESERVE must be a percentage from 0 to 100, not {value}"
                    )
                })?,
        };
        let seat = path.ancestors().find(|path| path.exists()).unwrap_or(path);
        let stats = fs4::statvfs(seat)
            .map_err(|error| format!("cannot read free space of {}: {error}", seat.display()))?;
        let (available, total) = (stats.available_space(), stats.total_space());
        Ok(Self {
            available,
            total,
            reserve,
            pressure: u128::from(available) * 100 < u128::from(total) * u128::from(reserve),
        })
    }

    pub fn span(&self) -> u64 {
        if self.pressure { 1 } else { SPAN }
    }
}
