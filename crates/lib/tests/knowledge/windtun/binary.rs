use super::material::Codec;
use flate2::{Compression, write::GzEncoder};
use plumb::identity::{Binding, Region};
use serde_json::{Value, json};

pub struct Binary;

impl Binary {
    pub fn image(source: &Value) -> Vec<u8> {
        Self::built(source, None)
    }

    pub fn built(source: &Value, commit: Option<&str>) -> Vec<u8> {
        let product = source["repository"]
            .as_str()
            .unwrap()
            .split('/')
            .next_back()
            .unwrap();
        let region = Region::new(
            &product.to_uppercase(),
            commit,
            Some("x86_64-unknown-linux-gnu"),
        )
        .read();
        let mut raw = vec![0; 4416];
        raw[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
        raw[16..18].copy_from_slice(&2_u16.to_le_bytes());
        raw[18..20].copy_from_slice(&62_u16.to_le_bytes());
        raw[20..24].copy_from_slice(&1_u32.to_le_bytes());
        raw[40..48].copy_from_slice(&4224_u64.to_le_bytes());
        raw[52..54].copy_from_slice(&64_u16.to_le_bytes());
        raw[58..60].copy_from_slice(&64_u16.to_le_bytes());
        raw[60..62].copy_from_slice(&3_u16.to_le_bytes());
        raw[62..64].copy_from_slice(&2_u16.to_le_bytes());
        raw[64..82].copy_from_slice(b"\0.relid\0.shstrtab\0");
        raw[128..4224].copy_from_slice(&region);
        let section = &mut raw[4288..4352];
        section[..4].copy_from_slice(&1_u32.to_le_bytes());
        section[4..8].copy_from_slice(&1_u32.to_le_bytes());
        section[8..16].copy_from_slice(&2_u64.to_le_bytes());
        section[24..32].copy_from_slice(&128_u64.to_le_bytes());
        section[32..40].copy_from_slice(&4096_u64.to_le_bytes());
        section[48..56].copy_from_slice(&8_u64.to_le_bytes());
        let strings = &mut raw[4352..4416];
        strings[..4].copy_from_slice(&8_u32.to_le_bytes());
        strings[4..8].copy_from_slice(&3_u32.to_le_bytes());
        strings[24..32].copy_from_slice(&64_u64.to_le_bytes());
        strings[32..40].copy_from_slice(&18_u64.to_le_bytes());
        strings[48..56].copy_from_slice(&1_u64.to_le_bytes());
        plumb::identity::bind(
            &raw,
            &Binding {
                product: product.into(),
                marker: source["marker"].as_str().unwrap().into(),
                commit: source["commit"].as_str().unwrap().into(),
                digest: Codec::digest(source),
                workload: "e".repeat(64),
            },
        )
        .unwrap()
    }

    pub fn archive(source: &Value) -> Vec<u8> {
        Self::pack(source, &Self::image(source))
    }

    pub fn pack(source: &Value, binary: &[u8]) -> Vec<u8> {
        let name = source["repository"]
            .as_str()
            .unwrap()
            .split('/')
            .next_back()
            .unwrap();
        let mut tar = tar::Builder::new(Vec::new());
        let mut header = tar::Header::new_gnu();
        header.set_size(binary.len() as u64);
        header.set_mode(0o755);
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(0);
        header.set_cksum();
        tar.append_data(&mut header, name, binary).unwrap();
        let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
        std::io::Write::write_all(&mut gzip, &tar.into_inner().unwrap()).unwrap();
        gzip.finish().unwrap()
    }

    pub fn source(product: &str, marker: &str) -> Value {
        let (commit, tree) = if product == "seatrial" {
            ("3", "4")
        } else {
            ("1", "2")
        };
        json!({"repository":format!("PerishLab/{product}"), "marker":marker, "commit":commit.repeat(40), "tree":tree.repeat(40)})
    }
}
