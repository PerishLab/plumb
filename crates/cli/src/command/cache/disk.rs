use serde::{Deserialize, Serialize};
use std::path::Path;

pub(super) const SPAN: u64 = 2;
const RESERVE: u64 = 30;
const KEY: &str = "PLUMB_GUARD_CACHE_RESERVE";

#[derive(Clone, Copy, Serialize)]
pub(super) struct Disk {
    pub available: u64,
    pub total: u64,
    pub reserve: u64,
    pub line: u64,
    pub deficit: u64,
    pub pressure: bool,
}

#[derive(Default, Deserialize)]
struct File {
    #[serde(default)]
    cache: Settings,
}

#[derive(Default, Deserialize)]
struct Settings {
    reserve: Option<u64>,
}

impl Disk {
    pub fn read(home: &Path, path: &Path) -> Result<Self, String> {
        let reserve = reserve(home)?;
        let seat = path.ancestors().find(|path| path.exists()).unwrap_or(path);
        let stats = fs4::statvfs(seat)
            .map_err(|error| format!("cannot read free space of {}: {error}", seat.display()))?;
        Ok(Self::measure(
            stats.available_space(),
            stats.total_space(),
            reserve,
        ))
    }

    fn measure(available: u64, total: u64, reserve: u64) -> Self {
        let line =
            u64::try_from((u128::from(total) * u128::from(reserve)).div_ceil(100)).unwrap_or(total);
        Self {
            available,
            total,
            reserve,
            line,
            deficit: line.saturating_sub(available),
            pressure: available < line,
        }
    }

    pub fn span(&self) -> u64 {
        if self.pressure { 1 } else { SPAN }
    }
}

fn reserve(home: &Path) -> Result<u64, String> {
    if let Some(value) = plumb::config::value(KEY) {
        return value
            .parse()
            .ok()
            .filter(|percent| *percent <= 100)
            .ok_or_else(|| format!("{KEY} must be a percentage from 0 to 100, not {value}"));
    }
    let path = home.join("plumb.toml");
    let file: File = match plumb::config::load(&path) {
        Ok(file) => file,
        Err(plumb::config::Error::Read { source, .. })
            if source.kind() == std::io::ErrorKind::NotFound =>
        {
            return Ok(RESERVE);
        }
        Err(error) => return Err(format!("{error}; check [cache] reserve")),
    };
    match file.cache.reserve {
        None => Ok(RESERVE),
        Some(percent) if percent <= 100 => Ok(percent),
        Some(percent) => Err(format!(
            "[cache] reserve in {} must be a percentage from 0 to 100, not {percent}",
            path.display()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::Disk;

    #[test]
    fn line() {
        let disk = Disk::measure(29, 128, 30);
        assert_eq!(disk.line, 39);
        assert_eq!(disk.deficit, 10);
        assert!(Disk::measure(38, 128, 30).pressure);
        assert!(!Disk::measure(39, 128, 30).pressure);
        assert!(disk.pressure);
        let calm = Disk::measure(40, 128, 30);
        assert_eq!(calm.deficit, 0);
        assert!(!calm.pressure);
        assert_eq!(Disk::measure(0, u64::MAX, 100).line, u64::MAX);
    }
}
