extern crate self as plumb;

pub mod cancel;
pub mod cli;
pub mod config;
pub mod context;
pub mod fill;
mod proof;
#[cfg(any(feature = "vendor", feature = "skill"))]
pub mod rig;
#[cfg(feature = "skill")]
pub mod skill;
#[cfg(feature = "vendor")]
pub mod vendor;

#[cfg(feature = "vendor")]
pub use vendor::forgejo::land;

#[cfg(feature = "radius")]
pub use proof::radius;

pub use proof::{boundary, trace, vocabulary};

pub use serde;

#[macro_export]
macro_rules! version {
    ($prefix:literal) => {
        option_env!(concat!($prefix, "_BUILD_VERSION"))
            .unwrap_or(concat!("v", env!("CARGO_PKG_VERSION")))
    };
}

#[macro_export]
macro_rules! commit {
    ($prefix:literal) => {
        option_env!(concat!($prefix, "_BUILD_COMMIT"))
    };
}

#[macro_export]
macro_rules! channel {
    ($prefix:literal) => {
        option_env!(concat!($prefix, "_BUILD_CHANNEL")).unwrap_or("dev")
    };
}
