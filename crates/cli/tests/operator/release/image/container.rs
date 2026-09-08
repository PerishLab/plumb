use std::path::Path;

pub(super) fn archive(path: &Path, count: usize) {
    let file = std::fs::File::create(path).expect("archive");
    let mut archive = tar::Builder::new(file);
    let body = serde_json::to_vec(&vec![serde_json::json!({"Config":"config.json"}); count])
        .expect("manifest");
    let mut header = tar::Header::new_gnu();
    header.set_mode(0o644);
    header.set_size(body.len() as u64);
    header.set_cksum();
    archive
        .append_data(&mut header, "manifest.json", body.as_slice())
        .expect("manifest");
    archive.finish().expect("archive");
}
