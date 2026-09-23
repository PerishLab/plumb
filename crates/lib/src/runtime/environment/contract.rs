use super::Contract;

const BASE: [&str; 15] = [
    "PATH",
    "HOME",
    "USERPROFILE",
    "SystemRoot",
    "SYSTEMROOT",
    "WINDIR",
    "COMSPEC",
    "PATHEXT",
    "TEMP",
    "TMP",
    "TMPDIR",
    "LANG",
    "LC_ALL",
    "LC_CTYPE",
    "TZ",
];

struct Held {
    inherit: &'static [&'static str],
    managed: &'static [&'static str],
    reject: &'static [&'static str],
}

const PNPM: Held = Held {
    inherit: &["PNPM_HOME"],
    managed: &[],
    reject: &[
        "NODE_OPTIONS",
        "NODE_PATH",
        "NODE_ENV",
        "NODE_EXTRA_CA_CERTS",
        "NODE_ICU_DATA",
        "NODE_PRESERVE_SYMLINKS",
        "NODE_TLS_REJECT_UNAUTHORIZED",
        "NODE_USE_ENV_PROXY",
        "NODE_USE_SYSTEM_CA",
        "NPM_CONFIG_*",
        "npm_config_*",
        "PNPM_*",
        "LD_PRELOAD",
        "DYLD_*",
    ],
};

const PROBE: Held = Held {
    inherit: &["PLUMB_HOME"],
    managed: &[],
    reject: &["LD_PRELOAD", "DYLD_*"],
};

const OCI: Held = Held {
    inherit: &["DOCKER_HOST"],
    managed: &["REGCTL_CONFIG", "DOCKER_AUTH_CONFIG"],
    reject: &["DOCKER_*", "BUILDKIT_*", "BUILDX_*", "LD_PRELOAD", "DYLD_*"],
};

const REGISTRY: Held = Held {
    inherit: &[],
    managed: &["REGCTL_CONFIG", "DOCKER_AUTH_CONFIG"],
    reject: &["REGCTL_*", "LD_PRELOAD", "DYLD_*"],
};

const CARGO: Held = Held {
    inherit: &[
        "CARGO_HOME",
        "RUSTUP_HOME",
        "RUSTUP_TOOLCHAIN",
        "LD_LIBRARY_PATH",
        "DYLD_LIBRARY_PATH",
        "DYLD_FALLBACK_LIBRARY_PATH",
    ],
    managed: &[
        "CARGO",
        "CARGO_TARGET_DIR",
        "CARGO_MANIFEST_*",
        "CARGO_PKG_*",
        "CARGO_REGISTRIES_*",
        "CARGO_BIN_EXE_*",
        "CARGO_CFG_*",
        "CARGO_FEATURE_*",
        "CARGO_PRIMARY_PACKAGE",
        "CARGO_MAKEFLAGS",
        "RUST_RECURSION_COUNT",
        "RUSTUP_TOOLCHAIN_SOURCE",
    ],
    reject: &[
        "CARGO_*",
        "RUST*",
        "CC",
        "CC_*",
        "CXX",
        "CXX_*",
        "AR",
        "AR_*",
        "CFLAGS",
        "CFLAGS_*",
        "CXXFLAGS",
        "CXXFLAGS_*",
        "CPPFLAGS",
        "LDFLAGS",
        "CMAKE_*",
        "PKG_CONFIG*",
        "LD_PRELOAD",
        "LD_LIBRARY_PATH",
        "DYLD_*",
    ],
};

impl Held {
    fn contract(&self) -> Contract {
        Contract {
            inherit: BASE
                .iter()
                .chain(self.inherit)
                .map(|name| (*name).to_string())
                .collect(),
            managed: self
                .managed
                .iter()
                .map(|name| (*name).to_string())
                .collect(),
            reject: self.reject.iter().map(|name| (*name).to_string()).collect(),
            bind: Default::default(),
        }
    }
}

pub fn contract(name: &str) -> Result<Contract, String> {
    let held = match name {
        "cargo" => &CARGO,
        "oci" => &OCI,
        "pnpm" => &PNPM,
        "probe" => &PROBE,
        "registry" => &REGISTRY,
        _ => return Err(format!("plumb runs nothing under a {name} contract")),
    };
    Ok(held.contract())
}
