pub(crate) mod cache;
pub mod cancel;
pub(crate) mod environment;
pub(crate) mod execution;
pub mod identity;
#[cfg(feature = "observation")]
pub mod observation;
pub(crate) mod process;
#[cfg(any(feature = "vendor", feature = "skill"))]
pub mod rig;

pub(crate) mod exchange {
    use std::time::{Duration, Instant};

    #[allow(dead_code)]
    pub(crate) fn https(
        operation: &'static str,
        outcome: &'static str,
        status: Option<u16>,
        elapsed: Duration,
    ) {
        attempt(operation, outcome, status, (1, elapsed));
    }

    #[allow(dead_code)]
    pub(crate) fn attempt(
        operation: &'static str,
        outcome: &'static str,
        status: Option<u16>,
        timing: (u8, Duration),
    ) {
        #[cfg(feature = "observation")]
        crate::runtime::observation::record(&crate::runtime::observation::Exchange {
            provider: "https",
            operation,
            transport: "https",
            outcome,
            status,
            attempts: timing.0,
            elapsed: timing.1,
        });
        #[cfg(not(feature = "observation"))]
        {
            let _ = (operation, outcome);
            let _ = (status, timing);
        }
    }

    #[allow(dead_code)]
    pub(crate) fn git(args: &[&str], started: Instant, ok: bool) {
        #[cfg(feature = "observation")]
        crate::runtime::observation::git(args, started, ok);
        #[cfg(not(feature = "observation"))]
        let _ = (args, started, ok);
    }
}
