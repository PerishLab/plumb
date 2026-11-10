use super::{Declaration, Home, Run, SEAT, finishes};
use locus::{Hook, Observation};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Capture(Mutex<Vec<String>>);

impl Hook for Capture {
    fn observe(&self, observation: &Observation) {
        if let Observation::Diagnostic(diagnostic) = observation {
            self.0
                .lock()
                .expect("diagnostics")
                .push(diagnostic.code().into());
        }
    }
}

fn open(home: &Home, capture: Arc<Capture>) -> Result<Option<Run>, String> {
    Run::open(
        Declaration {
            producer: "plumb",
            file: &home.file(),
            buffer: home.0.path().join("state/locus"),
        },
        "doctor",
        capture,
    )
}

#[test]
fn routed() {
    let _seat = SEAT.lock().expect("observation seat");
    let home = Home::new();
    let capture = Arc::new(Capture::default());
    let run = open(&home, capture.clone())
        .expect("configuration")
        .expect("run");
    run.fault("doctor.refused");
    run.finish(2);
    let codes = capture.0.lock().expect("diagnostics");
    assert_eq!(*codes, vec!["reporter.registration"]);
    assert_eq!(
        finishes(&home.records()),
        vec![(2.into(), "doctor.refused".into())]
    );
    assert!(plumb::observation::view().is_none());
}

#[test]
fn construction() {
    let _seat = SEAT.lock().expect("observation seat");
    let home = Home::new();
    std::fs::write(home.file(), "[locus]\nenabled=true\nendpoint='invalid'\n").unwrap();
    let capture = Arc::new(Capture::default());
    let error = open(&home, capture.clone())
        .err()
        .expect("construction refusal");
    assert!(error.contains("api endpoint"));
    assert_eq!(*capture.0.lock().unwrap(), vec!["bootstrap.failed"]);
    assert!(plumb::observation::view().is_none());
}

#[test]
fn configuration() {
    let _seat = SEAT.lock().expect("observation seat");
    let home = Home::new();
    let capture = Arc::new(Capture::default());
    std::fs::write(home.file(), "[locus]\nenabled=false\n").unwrap();
    assert!(open(&home, capture.clone()).unwrap().is_none());
    std::fs::remove_file(home.file()).unwrap();
    assert!(open(&home, capture.clone()).unwrap().is_none());
    assert!(!home.0.path().join("state/locus").exists());
    std::fs::write(home.file(), "[locus]\nenabled='invalid'\n").unwrap();
    assert!(
        open(&home, capture.clone())
            .err()
            .unwrap()
            .contains("invalid observation configuration")
    );
    assert!(capture.0.lock().unwrap().is_empty());
}
