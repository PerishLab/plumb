mod codec;
mod image;
mod region;
mod running;

pub use codec::{Binding, Codec, Origin};
pub use image::{bind, inspect};
pub use region::{Region, SIZE};
pub use running::{Reader, install, ready};

#[macro_export]
macro_rules! identity {
    ($prefix:literal) => {{
        #[used]
        #[cfg_attr(target_vendor = "apple", unsafe(link_section = "__DATA,__relid"))]
        #[cfg_attr(not(target_vendor = "apple"), unsafe(link_section = ".relid"))]
        static REGION: $crate::identity::Region = $crate::identity::Region::new(
            $prefix,
            option_env!(concat!($prefix, "_BUILD_COMMIT")),
            option_env!(concat!($prefix, "_BUILD_TARGET")),
        );
        $crate::identity::install(
            &REGION,
            option_env!(concat!($prefix, "_BUILD_CHANNEL")) == Some("unbound"),
        )
    }};
}
