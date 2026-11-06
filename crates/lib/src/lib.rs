extern crate self as plumb;

pub mod cli;
pub mod config;
#[path = "knowledge/cookbook/mod.rs"]
pub mod cookbook;
#[cfg(feature = "delivery")]
#[path = "forge/delivery/kernel.rs"]
pub mod delivery;
#[cfg(feature = "vendor")]
mod forge;
#[path = "proof/guard/integration/mod.rs"]
pub mod integration;
#[path = "forge/lane/mod.rs"]
pub mod lane;
#[path = "forge/packages/mod.rs"]
pub mod packages;
#[path = "forge/preview/mod.rs"]
pub mod preview;
#[path = "knowledge/semantics/mod.rs"]
pub mod semantics;
#[path = "knowledge/windtun.rs"]
pub mod windtun;
#[cfg(feature = "vendor")]
pub use forge::land;
pub use proof::guard::landing;
mod proof;
mod runtime;
pub mod seat;
#[cfg(feature = "testing")]
#[path = "knowledge/testing.rs"]
pub mod testing;
#[cfg(feature = "depot")]
pub use seat::bucket;
pub use seat::{depot, record, replace};
#[cfg(feature = "skill")]
pub mod skill;
#[cfg(feature = "vendor")]
pub mod vendor;

#[cfg(feature = "radius")]
pub use proof::radius;

pub use proof::{boundary, changelog, datum, guard, rule, snapshot, vocabulary};
#[cfg(feature = "observation")]
pub use runtime::observation;
#[cfg(any(feature = "vendor", feature = "skill"))]
pub use runtime::rig;
pub use runtime::{cancel, identity};

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
