extern crate self as plumb;

pub mod cancel;
pub mod cli;
pub mod config;
pub mod context;
pub mod trace;

pub use serde;

#[macro_export]
macro_rules! version {
    ($prefix:literal) => {
        option_env!(concat!($prefix, "_BUILD_VERSION"))
            .unwrap_or(concat!("v", env!("CARGO_PKG_VERSION")))
    };
}

#[macro_export]
macro_rules! channel {
    ($prefix:literal) => {
        option_env!(concat!($prefix, "_BUILD_CHANNEL")).unwrap_or("dev")
    };
}
