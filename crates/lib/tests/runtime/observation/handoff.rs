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

#[test]
fn settings() {
    use plumb::observation::Settings;
    assert_eq!(
        serde_json::from_value::<Settings>(serde_json::json!({})).unwrap(),
        Settings::default()
    );
    for value in [
        serde_json::json!({"enabled":"true"}),
        serde_json::json!({"endpoint":42}),
        serde_json::json!({"future":true}),
    ] {
        assert!(serde_json::from_value::<Settings>(value).is_err());
    }
    let settings = Settings {
        enabled: true,
        endpoint: Some("http://127.0.0.1:0".into()),
    };
    assert_eq!(
        serde_json::from_value::<Settings>(serde_json::to_value(&settings).unwrap()).unwrap(),
        settings
    );
}

#[test]
fn snapshot() {
    use plumb::observation::Settings;
    let _seat = SEAT.lock().expect("observation seat");
    let home = Home::new();
    std::fs::write(home.file(), "not a configuration").unwrap();
    unsafe {
        std::env::set_var("LOCUS_API", "invalid");
    }
    let capture = Arc::new(Capture::default());
    let file = home.file();
    let declaration = || Declaration {
        producer: "plumb",
        file: &file,
        buffer: home.0.path().join("state/locus"),
    };
    let disabled = Run::configured(
        declaration(),
        "doctor",
        Settings {
            enabled: false,
            endpoint: Some("invalid".into()),
        },
        capture.clone(),
    );
    assert!(disabled.unwrap().is_none());
    assert!(!home.0.path().join("state/locus").exists());
    assert!(capture.0.lock().unwrap().is_empty());
    let run = Run::configured(
        declaration(),
        "doctor",
        Settings {
            enabled: true,
            endpoint: Some("http://127.0.0.1:0".into()),
        },
        capture.clone(),
    )
    .unwrap()
    .unwrap();
    run.finish(0);
    unsafe {
        std::env::remove_var("LOCUS_API");
    }
    assert_eq!(*capture.0.lock().unwrap(), vec!["reporter.registration"]);
    assert_eq!(
        finishes(&home.records()),
        vec![(0.into(), serde_json::Value::Null)]
    );
    assert!(plumb::observation::view().is_none());
}

#[test]
fn legacy() {
    let _seat = SEAT.lock().expect("observation seat");
    let home = Home::new();
    std::fs::write(home.file(), "[locus]\nenabled=false\nfuture=true\n").unwrap();
    let capture = Arc::new(Capture::default());
    assert!(open(&home, capture.clone()).unwrap().is_none());
    assert!(capture.0.lock().unwrap().is_empty());
}
