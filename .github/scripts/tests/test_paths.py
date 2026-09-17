import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from controller import flags


class Paths(unittest.TestCase):
    def test_encoded_flags_keep_spaces_and_all_private_tool_roots(self):
        value = flags("/job/source", "/job/target", {
            "CARGO_HOME": "/job/cargo home", "RUSTUP_HOME": "/job/rustup",
            "RUSTUP_TOOLCHAIN": "/job/tools"})
        self.assertEqual(value.split("\x1f"), [
            "--remap-path-prefix=/job/source=/plumb/source",
            "--remap-path-prefix=/job/target=/plumb/target",
            "--remap-path-prefix=/job/cargo home=/plumb/cargo",
            "--remap-path-prefix=/job/rustup=/plumb/rustup",
            "--remap-path-prefix=/job/tools=/plumb/toolchain"])

    @unittest.skipUnless(sys.platform == "linux" and shutil.which("rustc"), "native Linux Rust acceptance")
    def test_registry_sources_in_independent_homes_produce_equal_executables(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            binaries = []
            for name in ("first home", "second home"):
                home = root / name
                source = home / "registry/src/main.rs"
                source.parent.mkdir(parents=True)
                source.write_text('fn main() { println!("{}", file!()); }')
                output = home / "probe"
                argv = [shutil.which("rustc"), str(source), "--crate-name", "probe",
                        "-C", "debuginfo=0", "-o", str(output)]
                argv.extend(flags(root / "source", root / "target", {"CARGO_HOME": str(home)}).split("\x1f"))
                subprocess.run(argv, check=True, capture_output=True, timeout=60, env=os.environ)
                actual = subprocess.check_output([str(output)], timeout=10).decode().strip()
                self.assertEqual(actual, "/plumb/cargo/registry/src/main.rs")
                binaries.append(output.read_bytes())
            self.assertEqual(binaries[0], binaries[1])
