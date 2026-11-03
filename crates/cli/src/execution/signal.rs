use signal_hook::consts::{SIGINT, SIGTERM};
use std::process::{Child, ExitStatus};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Once, OnceLock};
use std::time::Duration;

const GRACE: Duration = Duration::from_secs(2);
const POLL: Duration = Duration::from_millis(20);

static RECEIVED: OnceLock<Arc<AtomicUsize>> = OnceLock::new();
static WATCH: Once = Once::new();
static HELD: AtomicUsize = AtomicUsize::new(0);
static UNWIND: AtomicBool = AtomicBool::new(false);

pub(crate) struct Hold;

impl Hold {
    pub fn new() -> Result<Self, String> {
        HELD.fetch_add(1, Ordering::SeqCst);
        let held = Self;
        check()?;
        Ok(held)
    }
}

impl Drop for Hold {
    fn drop(&mut self) {
        HELD.fetch_sub(1, Ordering::SeqCst);
    }
}

pub(crate) fn check() -> Result<(), String> {
    match received() {
        Some(code) => {
            UNWIND.store(true, Ordering::SeqCst);
            Err(format!("execution interrupted by signal {}", code - 128))
        }
        None => Ok(()),
    }
}

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
            UNWIND.store(true, Ordering::SeqCst);
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
    while HELD.load(Ordering::SeqCst) != 0 {
        std::thread::sleep(POLL);
    }
    if plumb::integration::staging::held() {
        std::thread::sleep(GRACE);
        plumb::integration::staging::abandon();
    } else if UNWIND.load(Ordering::SeqCst) {
        return;
    }
    std::process::exit(code);
}

fn code(signal: usize) -> i32 {
    i32::try_from(signal).map_or(255, |signal| signal.saturating_add(128))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::Instant;

    struct Closing(PathBuf);

    impl Drop for Closing {
        fn drop(&mut self) {
            std::fs::write(&self.0, "closed").unwrap();
        }
    }

    struct Owner(Child);

    impl Drop for Owner {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    fn awaiter(mut ready: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !ready() {
            assert!(Instant::now() < deadline, "signal fixture did not progress");
            std::thread::sleep(POLL);
        }
    }

    fn child(root: &Path, cooperative: bool, staged: bool) {
        watch();
        let claim = staged.then(|| plumb::integration::staging::Claim::new(root, root).unwrap());
        if let Some(claim) = &claim {
            std::fs::write(
                root.join("claim"),
                claim.root.with_extension("lock").to_str().unwrap(),
            )
            .unwrap();
        }
        let closing = Closing(root.join("closed"));
        let held = cooperative.then(|| Hold::new().unwrap());
        std::fs::write(root.join("ready"), "ready").unwrap();
        if cooperative {
            awaiter(|| received().is_some());
            assert!(check().is_err());
            drop(held);
            std::fs::write(root.join("unwinding"), "unwinding").unwrap();
        }
        awaiter(|| root.join("release").exists());
        drop(closing);
        std::process::exit(received().unwrap());
    }

    fn scenario(name: &str, cooperative: bool, signal: &str) {
        if let Some(root) = std::env::var_os("PLUMB_SIGNAL_CHILD") {
            child(Path::new(&root), cooperative, name == "abandoned");
            return;
        }
        let root = tempfile::tempdir().unwrap();
        let mut owner = Owner(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    &format!("execution::signal::tests::{name}"),
                    "--nocapture",
                ])
                .env("PLUMB_SIGNAL_CHILD", root.path())
                .spawn()
                .unwrap(),
        );
        awaiter(|| {
            assert!(owner.0.try_wait().unwrap().is_none());
            root.path().join("ready").exists()
        });
        let sent = Instant::now();
        assert!(
            Command::new("kill")
                .args([signal, &owner.0.id().to_string()])
                .status()
                .unwrap()
                .success()
        );
        let finish = cooperative && name != "abandoned";
        if finish {
            awaiter(|| {
                assert!(
                    owner.0.try_wait().unwrap().is_none(),
                    "patrol exited before outer scopes finished"
                );
                root.path().join("unwinding").exists()
            });
            std::thread::sleep(Duration::from_millis(200));
            assert!(
                owner.0.try_wait().unwrap().is_none(),
                "patrol stole cooperative finalization"
            );
            std::fs::write(root.path().join("release"), "release").unwrap();
        }
        awaiter(|| owner.0.try_wait().unwrap().is_some());
        assert_eq!(
            owner.0.wait().unwrap().code(),
            Some(if signal == "-TERM" { 143 } else { 130 })
        );
        assert_eq!(root.path().join("closed").exists(), finish);
        if name == "abandoned" {
            assert!(sent.elapsed() >= GRACE);
            let claim = std::fs::read_to_string(root.path().join("claim")).unwrap();
            assert!(
                !Path::new(&claim).exists(),
                "fallback did not abandon staging"
            );
        }
    }

    #[test]
    fn cooperative() {
        for signal in ["-TERM", "-INT"] {
            scenario("cooperative", true, signal);
        }
    }

    #[test]
    fn uncooperative() {
        for signal in ["-TERM", "-INT"] {
            scenario("uncooperative", false, signal);
        }
    }

    #[test]
    fn abandoned() {
        for signal in ["-TERM", "-INT"] {
            scenario("abandoned", true, signal);
        }
    }
}
