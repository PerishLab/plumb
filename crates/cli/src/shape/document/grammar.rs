pub fn atom(value: &str) -> bool {
    let mut bytes = value.bytes();
    if !matches!(bytes.next(), Some(b'a'..=b'z')) {
        return false;
    }
    bytes.all(atomary)
}

pub fn seat(value: &str) -> bool {
    if value == "." {
        return true;
    }
    if value.is_empty() {
        return false;
    }
    if value.starts_with('/') || value.ends_with('/') {
        return false;
    }
    if value.contains('\\') {
        return false;
    }
    value.split('/').all(component)
}

pub fn overlaps(left: &str, right: &str) -> bool {
    if left == "." || right == "." {
        return true;
    }
    if left == right {
        return true;
    }
    right.starts_with(&format!("{left}/")) || left.starts_with(&format!("{right}/"))
}

fn atomary(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'
}

fn component(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    value != "." && value != ".."
}
