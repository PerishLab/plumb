#[cfg(unix)]
use super::capture;
use super::{CEILING, FLOOR, pace};
#[cfg(unix)]
use std::process::Command;
use std::time::Duration;

#[test]
fn pacing() {
    let mut pause = FLOOR;
    let mut steps = 0;
    while pause < CEILING {
        let next = pace(pause);
        assert!(next > pause);
        pause = next;
        steps += 1;
        assert!(steps < 16);
    }
    assert_eq!(pause, CEILING);
    assert_eq!(pace(CEILING), CEILING);
    assert!(FLOOR < Duration::from_millis(1));
}

#[cfg(unix)]
fn shell(script: &str) -> Command {
    let mut command = Command::new("sh");
    command.args(["-c", script]);
    command
}

#[cfg(unix)]
#[test]
fn output() {
    let bytes = capture(shell("printf fixture"), Duration::from_secs(30)).unwrap();
    assert_eq!(bytes, b"fixture");
}

#[cfg(unix)]
#[test]
fn refused() {
    let error = capture(shell("echo denied >&2; exit 3"), Duration::from_secs(30)).unwrap_err();
    assert!(error.contains("external command refused"), "{error}");
    assert!(error.contains("denied"), "{error}");
}

#[cfg(unix)]
#[test]
fn bound() {
    let error = capture(shell("sleep 30"), Duration::from_millis(300)).unwrap_err();
    assert!(
        error.contains("exceeded its time or output bound"),
        "{error}"
    );
}
