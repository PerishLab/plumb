extern crate self as plumb;

pub mod cli;
pub mod config;
pub mod fill;
#[cfg(feature = "vendor")]
pub mod forgejo;
mod proof;
mod runtime;
pub mod seat;
#[cfg(feature = "depot")]
pub use seat::bucket;
pub use seat::depot;
#[cfg(feature = "skill")]
pub mod skill;
#[cfg(feature = "vendor")]
pub mod vendor;

#[cfg(feature = "vendor")]
pub use forgejo::land;

#[cfg(feature = "radius")]
pub use proof::radius;

pub use proof::{boundary, changelog, datum, guard, rule, snapshot, vocabulary};
#[cfg(any(feature = "vendor", feature = "skill"))]
pub use runtime::rig;
pub use runtime::{cancel, context, identity, trace};

pub use serde;

#[macro_export]
macro_rules! version {
    ($prefix:literal) => {
        $crate::identity::Reader($prefix)
            .version()
            .unwrap_or_else(|| {
                option_env!(concat!($prefix, "_BUILD_VERSION"))
                    .unwrap_or(concat!("v", env!("CARGO_PKG_VERSION")))
            })
    };
}

#[macro_export]
macro_rules! commit {
    ($prefix:literal) => {
        $crate::identity::Reader($prefix)
            .commit()
            .or(option_env!(concat!($prefix, "_BUILD_COMMIT")))
    };
}

#[macro_export]
macro_rules! channel {
    ($prefix:literal) => {
        $crate::identity::Reader($prefix)
            .channel()
            .unwrap_or_else(|| option_env!(concat!($prefix, "_BUILD_CHANNEL")).unwrap_or("dev"))
    };
}
