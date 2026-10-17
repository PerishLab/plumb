use signal_hook::consts::{SIGINT, SIGTERM};
use std::process::{Child, ExitStatus};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Once, OnceLock};
use std::time::Duration;

const GRACE: Duration = Duration::from_secs(2);
const POLL: Duration = Duration::from_millis(20);

static RECEIVED: OnceLock<Arc<AtomicUsize>> = OnceLock::new();
static WATCH: Once = Once::new();

pub(crate) fn watch() {
    WATCH.call_once(|| {
        let received = RECEIVED.get_or_init(Arc::default);
        for signal in [SIGINT, SIGTERM] {
            let value = usize::try_from(signal).unwrap_or_default();
            if let Err(error) =
                signal_hook::flag::register_usize(signal, Arc::clone(received), value)
            {
                eprintln!("plumb: cannot handle signal {signal}: {error}");
            }
        }
        let received = Arc::clone(received);
        std::thread::spawn(move || patrol(&received));
    });
}

pub(crate) fn received() -> Option<i32> {
    let signal = RECEIVED.get()?.load(Ordering::SeqCst);
    (signal != 0).then(|| code(signal))
}

pub(crate) fn wait(child: &mut Child, argv: &[String]) -> Result<ExitStatus, String> {
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("cannot wait for {}: {error}", argv.join(" ")))?
        {
            return Ok(status);
        }
        if let Some(code) = received() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "{} interrupted by signal {}",
                argv.join(" "),
                code - 128
            ));
        }
        std::thread::sleep(POLL);
    }
}

fn patrol(received: &AtomicUsize) {
    while received.load(Ordering::SeqCst) == 0 {
        std::thread::sleep(POLL);
    }
    let code = code(received.load(Ordering::SeqCst));
    if plumb::integration::staging::held() {
        std::thread::sleep(GRACE);
        plumb::integration::staging::abandon();
    }
    std::process::exit(code);
}

fn code(signal: usize) -> i32 {
    i32::try_from(signal).map_or(255, |signal| signal.saturating_add(128))
}
