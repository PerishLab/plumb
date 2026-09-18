mod bootstrap;
mod changelog;
mod closure;
mod doctor;
mod workflow;

#[path = "../support.rs"]
mod support;

pub(crate) use doctor::{fixture, govern, ruled, run};
