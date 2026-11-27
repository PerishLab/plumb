use super::{changelog, release};
use crate::shape;
use crate::shape::release::Spec;
use clap::Subcommand;
use plumb::depot::v3::{Generation, Kind, Query};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const LEAF: &str = "SKILL.md";

#[derive(Subcommand)]
pub enum Deed {
    #[command(
        about = "Consign a release's changelog, skill, scene or configuration to wharf's yard and lodge it on Depot"
    )]
    Consign {
        #[command(flatten)]
        options: super::operator::Consign,
    },
    #[command(
        about = "Read the skill a release version lodged on Depot",
        long_about = plumb::seat::resource!("help/depot/skill.txt")
    )]
    Skill {
        #[command(flatten)]
        target: plumb::cli::Root,
        #[arg(
            long,
            help = "Release version to read (defaults to repository version)"
        )]
        version: Option<String>,
        #[arg(long, help = "Write only the exact verified SKILL.md bytes to stdout")]
        raw: bool,
    },
    #[command(about = "Export the standing Depot skill brief into a directory")]
    Export {
        #[command(flatten)]
        target: plumb::cli::Root,
        #[arg(long, value_parser = ["skill"])]
        kind: String,
        #[arg(
            long,
            help = "Release version to export (defaults to repository version)"
        )]
        version: Option<String>,
        #[arg(long, value_name = "DIR")]
        out: PathBuf,
    },
}

struct Brief {
    version: String,
    generation: String,
    body: String,
}

struct Seat<'a>(&'a Path);

pub fn run(deed: Deed) -> i32 {
    match deed {
        Deed::Consign { options } => match super::operator::consign(options) {
            Ok(message) => {
                println!("{message}");
                0
            }
            Err(error) => {
                eprintln!("plumb depot: {error}");
                1
            }
        },
        Deed::Skill {
            target,
            version,
            raw,
        } => match Seat(&PathBuf::from(target.root)).skill(version, raw) {
            Ok(()) => 0,
            Err(error) => {
                eprintln!("plumb depot: {error}");
                1
            }
        },
        Deed::Export {
            target,
            kind: _,
            version,
            out,
        } => match Seat(&PathBuf::from(target.root)).export(version, &out) {
            Ok(message) => {
                println!("{message}");
                0
            }
            Err(error) => {
                eprintln!("plumb depot: {error}");
                1
            }
        },
    }
}

impl Seat<'_> {
    fn export(&self, version: Option<String>, out: &Path) -> Result<String, String> {
        let brief = self.read(version)?;
        let path = store(out, &brief.body)?;
        Ok(format!(
            "exported {} {} ({} bytes) to {}",
            brief.version,
            brief.generation,
            brief.body.len(),
            path.display()
        ))
    }

    fn skill(&self, version: Option<String>, raw: bool) -> Result<(), String> {
        let brief = self.read(version)?;
        write(&brief, raw)
    }

    fn read(&self, version: Option<String>) -> Result<Brief, String> {
        let spec = Spec::controller(self.0)?;
        if !spec.skill {
            return Err(format!("{} declares no skill on Depot", spec.product));
        }
        let (marker, channel) = self.resolve(version)?;
        let source = spec.source();
        let generation = Generation::latest(Query {
            source: &source,
            product: &spec.product,
            channel: &channel,
            version: &marker,
            kind: Kind::Skill,
        })?
        .ok_or_else(|| format!("{source} carries no skill for {marker}"))?;
        if !generation
            .manifest
            .objects
            .iter()
            .any(|object| object.path == LEAF)
        {
            return Err(format!(
                "depot skill generation {} carries no {LEAF}",
                generation.pointer.generation
            ));
        }
        let bytes = generation.read(LEAF)?;
        let body = String::from_utf8(bytes)
            .map_err(|error| format!("skill object {LEAF} is not UTF-8: {error}"))?;
        Ok(Brief {
            version: marker,
            generation: generation.pointer.generation,
            body,
        })
    }

    fn resolve(&self, version: Option<String>) -> Result<(String, String), String> {
        let raw = match version {
            Some(held) => held.trim().to_string(),
            None => shape::version(self.0).ok_or_else(|| {
                "no version to read: the repository declares none, so pass --version".to_string()
            })?,
        };
        let marker = changelog::stamped(&raw);
        let channel = release::channel(&marker)?;
        Ok((marker, channel))
    }
}

fn store(out: &Path, body: &str) -> Result<PathBuf, String> {
    let path = out.join(LEAF);
    let filled = path.exists()
        && !std::fs::read_to_string(&path)
            .unwrap_or_default()
            .trim()
            .is_empty();
    if filled {
        return Err(format!(
            "refusing to overwrite non-empty {}",
            path.display()
        ));
    }
    std::fs::create_dir_all(out)
        .map_err(|error| format!("cannot create {}: {error}", out.display()))?;
    std::fs::write(&path, body)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    Ok(path)
}

fn write(brief: &Brief, raw: bool) -> Result<(), String> {
    let mut out = io::stdout();
    if raw {
        return out
            .write_all(brief.body.as_bytes())
            .and_then(|()| out.flush())
            .map_err(|error| format!("cannot write skill: {error}"));
    }
    writeln!(out, "{} {}", brief.version, brief.generation)
        .and_then(|()| writeln!(out))
        .and_then(|()| out.write_all(brief.body.as_bytes()))
        .and_then(|()| ending(&mut out, &brief.body))
        .and_then(|()| out.flush())
        .map_err(|error| format!("cannot write skill: {error}"))
}

fn ending(out: &mut impl Write, body: &str) -> io::Result<()> {
    if body.ends_with('\n') {
        Ok(())
    } else {
        writeln!(out)
    }
}

#[cfg(test)]
mod tests {
    use super::store;

    #[test]
    fn exports() {
        let dir = tempfile::tempdir().expect("dir");
        let path = store(dir.path(), "# Skill\n").expect("write");
        assert_eq!(std::fs::read_to_string(&path).expect("read"), "# Skill\n");
        let refused = store(dir.path(), "# Other\n").expect_err("no overwrite");
        assert!(refused.contains("refusing to overwrite"), "{refused}");
    }
}
