#[path = "../../src/vendor/forgejo/token/parser.rs"]
mod parser;

#[test]
fn single() {
    let config = "logins:\n- name: workshop\n  url: https://git.perish.top\n  token: secret\n  ssh_host: git.perish.top\n";
    assert_eq!(
        parser::login(config, "git.perish.top").as_deref(),
        Some("secret")
    );
}

#[test]
fn multiple() {
    let config = "logins:\n- name: other\n  url: https://other.test\n  token: first\n- name: workshop\n  url: https://git.perish.top\n  token: second\n";
    assert_eq!(
        parser::login(config, "git.perish.top").as_deref(),
        Some("second")
    );
}
