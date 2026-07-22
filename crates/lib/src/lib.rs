pub mod cli;
pub mod config;

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
