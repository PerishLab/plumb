use plumb::identity::Agent;

#[test]
fn value() {
    for version in ["v1.2.3", "v1.2.3-rc.1", "unbound", "0.0.0"] {
        let agent = Agent::new("santi-api", version).unwrap();
        assert_eq!(agent.value(), format!("santi-api/{version}"));
        assert_eq!(agent.to_string(), agent.value());
    }
}

#[test]
fn invalid() {
    for value in ["", "a b", "a/b", "a\r\nb", "a(b)", "你好", "a\0b"] {
        assert!(Agent::new(value, "v1.0.0").is_err());
        assert!(Agent::new("santi-api", value).is_err());
    }
}

#[test]
fn fallback() {
    let agent = plumb::agent!("ABSENT", "consumer").unwrap();
    assert_eq!(
        agent.value(),
        format!("consumer/{}", plumb::version!("ABSENT"))
    );
}
