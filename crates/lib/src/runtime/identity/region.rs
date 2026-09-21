pub const SIZE: usize = 4096;
pub(super) const MAGIC: &[u8; 16] = b"RELEASE.IDENT.V2";
pub(super) const PAYLOAD: usize = 288;

#[repr(transparent)]
pub struct Region([u8; SIZE]);

impl Region {
    pub const fn new(prefix: &str, commit: Option<&str>, target: Option<&str>) -> Self {
        let mut bytes = [0; SIZE];
        put(&mut bytes, 0, MAGIC);
        assert!(prefix.len() < 64);
        put(&mut bytes, 16, prefix.as_bytes());
        if let Some(commit) = commit {
            assert!(commit.len() <= 40);
            put(&mut bytes, 80, commit.as_bytes());
        }
        if let Some(target) = target {
            assert!(target.len() < 128);
            put(&mut bytes, 120, target.as_bytes());
        }
        Self(bytes)
    }

    pub fn read(&self) -> [u8; SIZE] {
        unsafe { std::ptr::read_volatile(&raw const self.0) }
    }
}

const fn put(bytes: &mut [u8; SIZE], offset: usize, value: &[u8]) {
    let mut index = 0;
    while index < value.len() {
        bytes[offset + index] = value[index];
        index += 1;
    }
}
