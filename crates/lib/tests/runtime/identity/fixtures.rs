use plumb::identity::{Binding, Codec, Origin};
use serde_json::Value;

const SOURCE: &str = "PerishLab/wharf resources/identity/fixtures at 90c61be, \
                      whose format is resources/identity/format.json";
const SIZE: usize = 4096;

const MANIFEST: &str = r#"{
  "bound.region": {
    "binding": {
      "commit": "3333333333333333333333333333333333333333",
      "digest": "2222222222222222222222222222222222222222222222222222222222222222",
      "marker": "v1.2.3-beta.4",
      "product": "demo-tool",
      "workload": "4444444444444444444444444444444444444444444444444444444444444444"
    },
    "origin": {
      "commit": "1111111111111111111111111111111111111111",
      "prefix": "DEMO_TOOL",
      "target": "x86_64-unknown-linux-gnu"
    }
  },
  "refused-checksum.region": {
    "refused": true
  },
  "refused-legacy-magic.region": {
    "refused": true
  },
  "refused-noncanonical-padding.region": {
    "refused": true
  },
  "refused-payload.region": {
    "refused": true
  },
  "refused-trailing-payload.region": {
    "refused": true
  },
  "refused-unbound-data.region": {
    "refused": true
  },
  "unbound.region": {
    "binding": null,
    "origin": {
      "commit": "1111111111111111111111111111111111111111",
      "prefix": "DEMO_TOOL",
      "target": "x86_64-unknown-linux-gnu"
    }
  }
}"#;

type Runs = &'static [(usize, &'static [u8])];

const REGIONS: &[(&str, Runs)] = &[
    (
        "bound.region",
        &[
            (0, b"RELEASE.IDENT.V2DEMO_TOO"),
            (24, b"L"),
            (80, b"111111111111111111111111"),
            (104, b"1111111111111111x86_64-u"),
            (128, b"nknown-linux-gnu"),
            (248, b"\xfe\x00\x00\x00\x00\x00\x00\x00\xa4\x8d\x99\x17*\x9e}\xd4\xa8\xe1hU\xdc\x17%\xb0"),
            (272, b"\xa6\xb9`\xa5\xa2\xb68\xf2\xc0\x5c\xa8\xc7\xfc\x08\xb0\x0f{\x22produc"),
            (296, b"t\x22:\x22demo-tool\x22,\x22marker\x22:"),
            (320, b"\x22v1.2.3-beta.4\x22,\x22digest\x22"),
            (344, b":\x222222222222222222222222"),
            (368, b"222222222222222222222222"),
            (392, b"222222222222222222\x22,\x22com"),
            (416, b"mit\x22:\x22333333333333333333"),
            (440, b"3333333333333333333333\x22,"),
            (464, b"\x22workload\x22:\x22444444444444"),
            (488, b"444444444444444444444444"),
            (512, b"444444444444444444444444"),
            (536, b"4444\x22}"),
        ],
    ),
    (
        "refused-checksum.region",
        &[
            (0, b"RELEASE.IDENT.V2DEMO_TOO"),
            (24, b"L"),
            (80, b"111111111111111111111111"),
            (104, b"1111111111111111x86_64-u"),
            (128, b"nknown-linux-gnu"),
            (248, b"\xfe\x00\x00\x00\x00\x00\x00\x00\xa5\x8d\x99\x17*\x9e}\xd4\xa8\xe1hU\xdc\x17%\xb0"),
            (272, b"\xa6\xb9`\xa5\xa2\xb68\xf2\xc0\x5c\xa8\xc7\xfc\x08\xb0\x0f{\x22produc"),
            (296, b"t\x22:\x22demo-tool\x22,\x22marker\x22:"),
            (320, b"\x22v1.2.3-beta.4\x22,\x22digest\x22"),
            (344, b":\x222222222222222222222222"),
            (368, b"222222222222222222222222"),
            (392, b"222222222222222222\x22,\x22com"),
            (416, b"mit\x22:\x22333333333333333333"),
            (440, b"3333333333333333333333\x22,"),
            (464, b"\x22workload\x22:\x22444444444444"),
            (488, b"444444444444444444444444"),
            (512, b"444444444444444444444444"),
            (536, b"4444\x22}"),
        ],
    ),
    (
        "refused-legacy-magic.region",
        &[
            (0, b"PLUMB.IDENTITY.1DEMO_TOO"),
            (24, b"L"),
            (80, b"111111111111111111111111"),
            (104, b"1111111111111111x86_64-u"),
            (128, b"nknown-linux-gnu"),
        ],
    ),
    (
        "refused-noncanonical-padding.region",
        &[
            (0, b"RELEASE.IDENT.V2DEMO_TOO"),
            (24, b"L"),
            (70, b"X\x00\x00\x00\x00\x00\x00\x00\x00\x0011111111111111"),
            (94, b"111111111111111111111111"),
            (118, b"11x86_64-unknown-linux-g"),
            (142, b"nu"),
        ],
    ),
    (
        "refused-payload.region",
        &[
            (0, b"RELEASE.IDENT.V2DEMO_TOO"),
            (24, b"L"),
            (80, b"111111111111111111111111"),
            (104, b"1111111111111111x86_64-u"),
            (128, b"nknown-linux-gnu"),
            (248, b"\xfe\x00\x00\x00\x00\x00\x00\x00\xa4\x8d\x99\x17*\x9e}\xd4\xa8\xe1hU\xdc\x17%\xb0"),
            (272, b"\xa6\xb9`\xa5\xa2\xb68\xf2\xc0\x5c\xa8\xc7\xfc\x08\xb0\x0f{\x22psoduc"),
            (296, b"t\x22:\x22demo-tool\x22,\x22marker\x22:"),
            (320, b"\x22v1.2.3-beta.4\x22,\x22digest\x22"),
            (344, b":\x222222222222222222222222"),
            (368, b"222222222222222222222222"),
            (392, b"222222222222222222\x22,\x22com"),
            (416, b"mit\x22:\x22333333333333333333"),
            (440, b"3333333333333333333333\x22,"),
            (464, b"\x22workload\x22:\x22444444444444"),
            (488, b"444444444444444444444444"),
            (512, b"444444444444444444444444"),
            (536, b"4444\x22}"),
        ],
    ),
    (
        "refused-trailing-payload.region",
        &[
            (0, b"RELEASE.IDENT.V2DEMO_TOO"),
            (24, b"L"),
            (80, b"111111111111111111111111"),
            (104, b"1111111111111111x86_64-u"),
            (128, b"nknown-linux-gnu"),
            (248, b"\xfe\x00\x00\x00\x00\x00\x00\x00\xa4\x8d\x99\x17*\x9e}\xd4\xa8\xe1hU\xdc\x17%\xb0"),
            (272, b"\xa6\xb9`\xa5\xa2\xb68\xf2\xc0\x5c\xa8\xc7\xfc\x08\xb0\x0f{\x22produc"),
            (296, b"t\x22:\x22demo-tool\x22,\x22marker\x22:"),
            (320, b"\x22v1.2.3-beta.4\x22,\x22digest\x22"),
            (344, b":\x222222222222222222222222"),
            (368, b"222222222222222222222222"),
            (392, b"222222222222222222\x22,\x22com"),
            (416, b"mit\x22:\x22333333333333333333"),
            (440, b"3333333333333333333333\x22,"),
            (464, b"\x22workload\x22:\x22444444444444"),
            (488, b"444444444444444444444444"),
            (512, b"444444444444444444444444"),
            (536, b"4444\x22}"),
            (4095, b"\x01"),
        ],
    ),
    (
        "refused-unbound-data.region",
        &[
            (0, b"RELEASE.IDENT.V2DEMO_TOO"),
            (24, b"L"),
            (80, b"111111111111111111111111"),
            (104, b"1111111111111111x86_64-u"),
            (128, b"nknown-linux-gnu"),
            (300, b"\x01"),
        ],
    ),
    (
        "unbound.region",
        &[
            (0, b"RELEASE.IDENT.V2DEMO_TOO"),
            (24, b"L"),
            (80, b"111111111111111111111111"),
            (104, b"1111111111111111x86_64-u"),
            (128, b"nknown-linux-gnu"),
        ],
    ),
];

fn region(name: &str) -> Vec<u8> {
    let (_, runs) = REGIONS
        .iter()
        .find(|(held, _)| *held == name)
        .unwrap_or_else(|| panic!("{name} is no region of {SOURCE}"));
    let mut bytes = vec![0; SIZE];
    for (offset, run) in *runs {
        bytes[*offset..offset + run.len()].copy_from_slice(run);
    }
    bytes
}

fn text(value: &Value, field: &str) -> String {
    value[field].as_str().unwrap().to_owned()
}

#[test]
fn fixtures() {
    let manifest: Value = serde_json::from_str(MANIFEST).unwrap();
    let cases = manifest.as_object().unwrap();
    assert!(!cases.is_empty());
    let mut named = cases.keys().map(String::as_str).collect::<Vec<_>>();
    named.sort_unstable();
    let held = REGIONS.iter().map(|(name, _)| *name).collect::<Vec<_>>();
    assert_eq!(named, held, "every region of {SOURCE} is judged");
    for (name, expect) in cases {
        let bytes = region(name);
        let decoded = Codec(&bytes).decode();
        if expect.get("refused").is_some() {
            assert!(decoded.is_err(), "{name} must be refused");
            continue;
        }
        let (origin, binding) = decoded.unwrap_or_else(|error| panic!("{name}: {error}"));
        let held = &expect["origin"];
        assert_eq!(
            origin,
            Origin {
                prefix: text(held, "prefix"),
                commit: text(held, "commit"),
                target: text(held, "target"),
            },
            "{name}"
        );
        let wanted = match &expect["binding"] {
            Value::Null => None,
            held => Some(Binding {
                product: text(held, "product"),
                marker: text(held, "marker"),
                digest: text(held, "digest"),
                commit: text(held, "commit"),
                workload: text(held, "workload"),
            }),
        };
        assert_eq!(binding, wanted, "{name}");
        if let Some(wanted) = wanted {
            let unbound = region("unbound.region");
            assert_eq!(Codec(&unbound).encode(&wanted).unwrap(), bytes, "{name}");
        }
    }
}
