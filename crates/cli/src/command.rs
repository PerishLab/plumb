pub mod depot;
pub mod doctor;
mod guard;
pub mod operator;
pub use release::authority;
pub mod release;
pub mod retire;
pub mod ship;

pub(crate) use guard::{audit, changelog};
pub use guard::{clock, cookbook, land, precommit, radius, render, workflow};

impl crate::Command {
    pub(crate) fn name(&self) -> &'static str {
        match self {
            crate::Command::Authority { .. } => "authority",
            crate::Command::Doctor { .. } => "doctor",
            crate::Command::Land { .. } => "land",
            crate::Command::Guard { .. } => "guard",
            crate::Command::Radius { .. } => "radius",
            crate::Command::Policy { .. } => "policy",
            crate::Command::Skill { .. } => "skill",
            crate::Command::Rule { .. } => "rule",
            crate::Command::Changelog { .. } => "changelog",
            crate::Command::Configuration { .. } => "configuration",
            crate::Command::Layout { .. } => "layout",
            crate::Command::Cookbook { .. } => "cookbook",
            crate::Command::Affirm { .. } => "affirm",
            crate::Command::Depot { .. } => "depot",
            crate::Command::Release { .. } => "release",
            crate::Command::Ship { .. } => "ship",
            crate::Command::Retire { .. } => "retire",
            crate::Command::Workflow { .. } => "workflow",
        }
    }
}
