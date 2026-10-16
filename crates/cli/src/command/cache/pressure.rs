use super::disk::Disk;
use super::sweep::{self, Entry, Status};
use std::path::{Path, PathBuf};

pub(super) struct Candidate {
    pub used: u64,
    pub bytes: u64,
}

pub(super) fn select(candidates: &[Candidate], deficit: u64) -> Vec<usize> {
    let mut order: Vec<usize> = (0..candidates.len())
        .filter(|index| candidates[*index].bytes > 0)
        .collect();
    order.sort_by_key(|index| (candidates[*index].used, *index));
    let mut chosen = Vec::new();
    let mut covered = 0u64;
    for index in order {
        if covered >= deficit {
            break;
        }
        covered = covered.saturating_add(candidates[index].bytes);
        chosen.push(index);
    }
    chosen
}

pub(super) struct Relief<'a> {
    pub home: &'a Path,
    pub root: &'a Path,
    pub apply: bool,
    pub keep: Option<&'a Path>,
}

impl Relief<'_> {
    pub fn run(&self, disk: Disk, held: &mut [(PathBuf, Entry)]) -> Result<(), String> {
        if !disk.pressure {
            return Ok(());
        }
        let open: Vec<usize> = (0..held.len())
            .filter(|index| {
                let (path, entry) = &held[*index];
                entry.status == Status::Live
                    && entry.refusal.is_none()
                    && self.keep.is_none_or(|keep| keep != path.as_path())
            })
            .collect();
        let candidates: Vec<Candidate> = open
            .iter()
            .map(|index| {
                let entry = &held[*index].1;
                Candidate {
                    used: entry.used.unwrap_or(0),
                    bytes: entry.bytes.saturating_sub(entry.reclaimable),
                }
            })
            .collect();
        if !self.apply {
            let trimmed: u64 = held.iter().map(|(_, entry)| entry.reclaimable).sum();
            for chosen in select(&candidates, disk.deficit.saturating_sub(trimmed)) {
                let (path, entry) = &mut held[open[chosen]];
                sweep::evict(self.home, path, false, entry);
            }
            return Ok(());
        }
        for chosen in select(&candidates, u64::MAX) {
            if !Disk::read(self.home, self.root)?.pressure {
                break;
            }
            let (path, entry) = &mut held[open[chosen]];
            sweep::evict(self.home, path, true, entry);
        }
        Ok(())
    }
}

pub(super) fn recent(held: &[(PathBuf, Entry)]) -> Option<PathBuf> {
    held.iter()
        .filter(|(_, entry)| matches!(entry.status, Status::Live | Status::Active))
        .max_by_key(|(_, entry)| entry.used)
        .map(|(path, _)| path.clone())
}

#[cfg(test)]
mod tests {
    use super::{Candidate, select};

    fn held(pairs: &[(u64, u64)]) -> Vec<Candidate> {
        pairs
            .iter()
            .map(|(used, bytes)| Candidate {
                used: *used,
                bytes: *bytes,
            })
            .collect()
    }

    #[test]
    fn oldest() {
        let candidates = held(&[(30, 5), (10, 4), (20, 0), (15, 3)]);
        assert_eq!(select(&candidates, 0), Vec::<usize>::new());
        assert_eq!(select(&candidates, 4), vec![1]);
        assert_eq!(select(&candidates, 5), vec![1, 3]);
        assert_eq!(select(&candidates, 100), vec![1, 3, 0]);
    }
}
