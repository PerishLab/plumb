use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

pub(in crate::command::cache) const WINDOW: Duration = Duration::from_secs(10);
const FLOOR: f64 = 1_048_576.0;
const TICK: Duration = Duration::from_millis(250);

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::command::cache) enum Verdict {
    Stalled,
    Projected,
}

#[derive(Debug, PartialEq)]
pub(in crate::command::cache) struct Decision {
    pub verdict: Verdict,
    pub bytes: u64,
    pub rate: u64,
}

pub(in crate::command::cache) struct Watch {
    expected: u64,
    window: Duration,
    samples: Vec<(Duration, u64)>,
}

impl Watch {
    pub fn new(expected: u64, window: Duration) -> Self {
        Self {
            expected,
            window,
            samples: Vec::new(),
        }
    }

    pub fn note(&mut self, now: Duration, bytes: u64) {
        self.samples.push((now, bytes));
    }

    pub fn judge(&self, left: Duration) -> Option<Decision> {
        let &(now, bytes) = self.samples.last()?;
        let edge = now.checked_sub(self.window)?;
        let &(then, before) = self.samples.iter().rev().find(|(time, _)| *time <= edge)?;
        let span = (now - then).as_secs_f64();
        let rate = bytes.saturating_sub(before) as f64 / span;
        let decision = |verdict| Decision {
            verdict,
            bytes,
            rate: rate as u64,
        };
        if rate < FLOOR {
            return Some(decision(Verdict::Stalled));
        }
        let remaining = self.expected.saturating_sub(bytes) as f64;
        (remaining / rate > left.as_secs_f64()).then(|| decision(Verdict::Projected))
    }
}

pub(in crate::command::cache) fn follow(
    mut command: Command,
    target: &Path,
    mut watch: Watch,
    left: &dyn Fn() -> Result<Duration, String>,
) -> Result<Output, String> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "cannot run isolated cache read transport")?;
    let started = Instant::now();
    loop {
        if child
            .try_wait()
            .map_err(|_| "cannot watch isolated cache read transport")?
            .is_some()
        {
            return child
                .wait_with_output()
                .map_err(|_| "cannot read isolated cache read transport".into());
        }
        std::thread::sleep(TICK);
        let bytes = std::fs::metadata(target)
            .map(|meta| meta.len())
            .unwrap_or(0);
        watch.note(started.elapsed(), bytes);
        let remaining = match left() {
            Ok(time) => time,
            Err(error) => return stop(&mut child, error),
        };
        if let Some(decision) = watch.judge(remaining) {
            return stop(&mut child, refusal(&decision, &watch, remaining));
        }
    }
}

fn stop(child: &mut std::process::Child, error: String) -> Result<Output, String> {
    let _ = child.kill();
    let _ = child.wait();
    Err(error)
}

fn refusal(decision: &Decision, watch: &Watch, remaining: Duration) -> String {
    let (expected, window) = (watch.expected, watch.window);
    let reason = match decision.verdict {
        Verdict::Stalled => "stalled",
        Verdict::Projected => "projected",
    };
    crate::diagnostic::event("material.lookup.budget", || {
        serde_json::json!({
            "reason":reason,"bytes":decision.bytes,"expected":expected,
            "rate":decision.rate,"left":remaining.as_secs()
        })
    });
    match decision.verdict {
        Verdict::Stalled => format!(
            "cache artifact download stalled at {} of {expected} bytes ({} B/s over {}s)",
            decision.bytes,
            decision.rate,
            window.as_secs()
        ),
        Verdict::Projected => format!(
            "cache artifact download projected to miss the restore limit: {} of {expected} bytes at {} B/s with {}s left",
            decision.bytes,
            decision.rate,
            remaining.as_secs()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(rates: &[(u64, u64)]) -> Watch {
        let mut watch = Watch::new(1_000_000_000, WINDOW);
        let mut bytes = 0;
        let mut now = 0;
        watch.note(Duration::ZERO, 0);
        for &(seconds, rate) in rates {
            for _ in 0..seconds {
                now += 1;
                bytes += rate;
                watch.note(Duration::from_secs(now), bytes);
            }
        }
        watch
    }

    #[test]
    fn early() {
        let watch = feed(&[(9, 0)]);
        assert_eq!(watch.judge(Duration::from_secs(80)), None);
    }

    #[test]
    fn stalled() {
        let watch = feed(&[(5, 40_000_000), (10, 0)]);
        let decision = watch.judge(Duration::from_secs(60)).unwrap();
        assert_eq!(decision.verdict, Verdict::Stalled);
        assert_eq!(decision.bytes, 200_000_000);
    }

    #[test]
    fn projected() {
        let watch = feed(&[(12, 10_000_000)]);
        let decision = watch.judge(Duration::from_secs(70)).unwrap();
        assert_eq!(decision.verdict, Verdict::Projected);
        assert_eq!(decision.rate, 10_000_000);
        assert_eq!(watch.judge(Duration::from_secs(100)), None);
    }

    #[test]
    fn steady() {
        let watch = feed(&[(12, 20_000_000)]);
        assert_eq!(watch.judge(Duration::from_secs(60)), None);
    }

    #[test]
    fn recent() {
        let watch = feed(&[(10, 30_000_000), (12, 15_000_000)]);
        let decision = watch.judge(Duration::from_secs(20)).unwrap();
        assert_eq!(decision.verdict, Verdict::Projected);
        assert_eq!(decision.rate, 15_000_000);
    }

    fn script(text: &str) -> (Command, tempfile::NamedTempFile) {
        let target = tempfile::NamedTempFile::new().unwrap();
        let mut command = Command::new("sh");
        command.arg("-c").arg(text).env("TARGET", target.path());
        (command, target)
    }

    #[test]
    fn killed() {
        let (command, target) = script("head -c 3000000 /dev/zero > \"$TARGET\"; sleep 30");
        let started = Instant::now();
        let error = follow(
            command,
            target.path(),
            Watch::new(9_000_000, Duration::from_secs(1)),
            &|| Ok(Duration::from_secs(60)),
        )
        .unwrap_err();
        assert!(error.contains("stalled at 3000000 of 9000000"), "{error}");
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[test]
    fn complete() {
        let (command, target) = script("head -c 100 /dev/zero > \"$TARGET\"; printf 200");
        let output = follow(
            command,
            target.path(),
            Watch::new(100, Duration::from_secs(1)),
            &|| Ok(Duration::from_secs(60)),
        )
        .unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, b"200");
    }

    #[test]
    fn expired() {
        let (command, target) = script("sleep 30");
        let error = follow(command, target.path(), Watch::new(100, WINDOW), &|| {
            Err("cache restore exceeded its 1s budget".into())
        })
        .unwrap_err();
        assert!(error.contains("exceeded its 1s budget"), "{error}");
    }

    #[test]
    fn finished() {
        let mut watch = Watch::new(100_000_000, WINDOW);
        watch.note(Duration::ZERO, 0);
        watch.note(Duration::from_secs(11), 100_000_000);
        assert_eq!(watch.judge(Duration::from_secs(1)), None);
    }
}
