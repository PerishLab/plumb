use object::{Object, ObjectSection};
use plumb::identity::{Binding, Region};
use std::{fs, process::Command};

#[used]
#[cfg_attr(target_vendor = "apple", unsafe(link_section = "__DATA,__releaseid"))]
#[cfg_attr(not(target_vendor = "apple"), unsafe(link_section = ".releaseid"))]
static REGION: Region = Region::new(
    "TEST",
    Some("1111111111111111111111111111111111111111"),
    Some("test-target"),
);

fn binding() -> Binding {
    Binding {
        product: "test".into(),
        marker: "v1.2.3-beta.1".into(),
        digest: "2".repeat(64),
        commit: "3".repeat(40),
        workload: "4".repeat(64),
    }
}

fn executable() -> Vec<u8> {
    fs::read(std::env::current_exe().unwrap()).unwrap()
}

#[test]
fn probe() {
    plumb::identity::install(&REGION, true).unwrap();
    println!(
        "identity={} channel={} ready={}",
        plumb::identity::Reader("TEST").version().unwrap(),
        plumb::identity::Reader("TEST").channel().unwrap(),
        plumb::identity::ready().is_ok()
    );
}

#[test]
fn roundtrip() {
    let original = executable();
    let (origin, held) = plumb::identity::inspect(&original).unwrap();
    assert!(held.is_none());
    assert_eq!(origin.commit, "1".repeat(40));
    let candidate = binding();
    let bound = plumb::identity::bind(&original, &candidate).unwrap();
    let (after, held) = plumb::identity::inspect(&bound).unwrap();
    assert_eq!(after, origin);
    assert_eq!(held, Some(candidate.clone()));
    assert_eq!(plumb::identity::bind(&bound, &candidate).unwrap(), bound);
    assert_eq!(plumb::identity::inspect(&original).unwrap().1, None);
    let mut stable = candidate.clone();
    stable.marker = "v1.2.3".into();
    assert!(plumb::identity::bind(&bound, &stable).is_err());
    assert!(plumb::identity::bind(&original, &stable).is_ok());
}

#[test]
#[cfg(target_os = "linux")]
fn runs() {
    let original = executable();
    let target = tempfile::tempdir().unwrap();
    for marker in ["v1.2.3-beta.1", "v1.2.3"] {
        let mut identity = binding();
        identity.marker = marker.into();
        let bound = plumb::identity::bind(&original, &identity).unwrap();
        let path = target.path().join(marker);
        fs::write(&path, bound).unwrap();
        fs::set_permissions(
            &path,
            fs::metadata(std::env::current_exe().unwrap())
                .unwrap()
                .permissions(),
        )
        .unwrap();
        let result = Command::new(path)
            .args(["--exact", "probe", "--nocapture"])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let stdout = String::from_utf8(result.stdout).unwrap();
        assert!(
            stdout.contains(&format!(
                "identity={marker} channel={} ready=true",
                identity.channel().unwrap()
            )),
            "{stdout}"
        );
    }
}

#[test]
fn refuses() {
    let original = executable();
    let mut invalid = binding();
    invalid.product = "other".into();
    assert!(plumb::identity::bind(&original, &invalid).is_err());
    invalid = binding();
    invalid.digest = "x".repeat(64);
    assert!(plumb::identity::bind(&original, &invalid).is_err());
    invalid = binding();
    invalid.marker = "v1.2.3-dev".into();
    assert!(plumb::identity::bind(&original, &invalid).is_err());
    assert!(plumb::identity::inspect(b"not an executable").is_err());
    let mut bound = plumb::identity::bind(&original, &binding()).unwrap();
    let image = object::File::parse(bound.as_slice()).unwrap();
    let section = image
        .sections()
        .find(|section| matches!(section.name(), Ok(".releaseid" | "__releaseid")))
        .unwrap();
    let location = section.file_range().unwrap().0 as usize;
    bound[location + 288] ^= 1;
    assert!(plumb::identity::inspect(&bound).is_err());
}
