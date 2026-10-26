use serde::Deserialize;

use super::Spec;

#[derive(Clone, Debug)]
pub struct Target {
    pub triple: String,
}

pub(super) const TARGETS: [&str; 3] = [
    "x86_64-unknown-linux-gnu",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
];

pub(super) fn resolve(triple: &str) -> Result<Target, String> {
    if TARGETS.contains(&triple) {
        Ok(Target {
            triple: triple.into(),
        })
    } else {
        Err(format!(
            "unsupported release target {triple}; wharf releases {}",
            TARGETS.join(", ")
        ))
    }
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Windows {
    pub scope: Scope,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Scope {
    User,
    Machine,
}

impl Windows {
    pub(super) fn validate(&self, spec: &Spec) -> Result<(), String> {
        let installed = spec
            .executables
            .iter()
            .any(|binary| binary.install && binary.carries("x86_64-pc-windows-msvc"));
        if !installed {
            return Err(format!(
                "Windows {:?} installation requires an installed x86_64-pc-windows-msvc executable",
                self.scope
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Scope, Spec};
    use std::path::Path;

    const HEAD: &str = "[identity]\nns = \"demo\"\n[release]\nauthority = \"https://example.invalid\"\nbinaries = [\"demo\"]\ntargets = [\"x86_64-pc-windows-msvc\"]\n";

    #[test]
    fn scopes() {
        for (value, scope) in [("user", Scope::User), ("machine", Scope::Machine)] {
            let text = format!("{HEAD}[release.windows]\nscope = \"{value}\"\n");
            let spec = Spec::decode(Path::new("."), &text, "fixture").expect(value);
            assert_eq!(spec.windows.expect("Windows").scope, scope);
        }
        let spec = Spec::decode(Path::new("."), HEAD, "fixture").expect("unspecified");
        assert!(spec.windows.is_none());
    }

    #[test]
    fn refuses() {
        for table in [
            "[release.windows]\n",
            "[release.windows]\nscope = \"global\"\n",
            "[release.windows]\nscope = \"machine\"\nroot = \"C:/data\"\n",
            "[release.windows]\nscope = false\n",
        ] {
            assert!(Spec::decode(Path::new("."), &format!("{HEAD}{table}"), "fixture").is_err());
        }
        for head in [
            HEAD.replace("x86_64-pc-windows-msvc", "x86_64-unknown-linux-gnu"),
            format!("{HEAD}[release.binary.demo]\ninstall = false\n"),
        ] {
            let text = format!("{head}[release.windows]\nscope = \"machine\"\n");
            let error =
                Spec::decode(Path::new("."), &text, "fixture").expect_err("no Windows install");
            assert!(error.contains("requires an installed"), "{error}");
        }
    }
}
