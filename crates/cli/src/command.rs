pub(crate) mod cache;
pub mod depot;
pub mod doctor;
mod guard;
pub mod operator;
pub(crate) mod packages;
pub mod release;
pub mod ship;

pub(crate) use guard::changelog;
pub use guard::{cookbook, land, precommit, radius, render};

use crate::Command;
use std::path::PathBuf;

#[locus::trace(with = crate::observation::view())]
pub(crate) fn execute(command: Command) -> i32 {
    match command {
        Command::Lane { deed } => crate::consumption::lane::run(deed),
        Command::Follow {
            target,
            github,
            json,
            retire,
        } => {
            let input = packages::follow::Input {
                root: PathBuf::from(target.root),
                github,
                json,
            };
            if retire {
                packages::follow::retire(input)
            } else {
                packages::follow::run(input)
            }
        }
        Command::Lift { target, json } => packages::lift(PathBuf::from(target.root), json),
        Command::Doctor { target, json } => doctor::run(PathBuf::from(target.root), json),
        Command::Land {
            target,
            base,
            title,
            body,
            watch,
            dry,
            json,
        } => land::run(land::Input {
            root: PathBuf::from(target.root),
            base,
            title,
            body,
            watch,
            dry,
            json,
        }),
        Command::Guard {
            target,
            base,
            head,
            write,
            attach,
            refresh,
            json,
            diagnostic: _,
        } => precommit::run(precommit::Input {
            root: PathBuf::from(target.root),
            base,
            head,
            write,
            attach,
            refresh,
            json,
        }),
        Command::Radius {
            root,
            product,
            candidate,
            json,
        } => radius::run(radius::Input {
            roots: root,
            product,
            candidate,
            json,
        }),
        Command::Policy { target, write } => {
            render::Seat::new(PathBuf::from(target.root)).policy(write)
        }
        Command::Skill { deed } => crate::consumption::skill::run(deed),
        Command::Rule { deed } => crate::catalog::query::run(deed),
        Command::Changelog {
            target,
            version,
            prove: Some(home),
        } => render::Seat::new(PathBuf::from(target.root)).proven(version, &home),
        Command::Changelog {
            target, version, ..
        } => render::Seat::new(PathBuf::from(target.root)).changelog(version),
        Command::Configuration { deed } => crate::consumption::configuration::run(deed),
        Command::Layout { target } => render::Seat::new(PathBuf::from(target.root)).layout(),
        Command::Metadata { key, json } => crate::consumption::metadata::run(key, json),
        Command::Cookbook { entry, json } => cookbook::run(entry, json),
        Command::Affirm { target, write } => {
            render::Seat::new(PathBuf::from(target.root)).affirm(write)
        }
        Command::Release { deed } => release::run(deed),
        Command::Ship { deed } => ship::run(deed),
        Command::Depot { deed } => depot::run(deed),
        Command::Cache { deed } => cache::run(deed),
    }
}
