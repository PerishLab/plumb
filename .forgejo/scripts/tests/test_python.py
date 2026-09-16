import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


@unittest.skipUnless(shutil.which("pwsh"), "PowerShell is required for bootstrap execution tests")
class PythonBootstrap(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.shell = shutil.which("pwsh")
        self.script = Path(__file__).resolve().parents[1] / "python.ps1"
        self.environment = dict(os.environ, RUNNER_TEMP=str(self.root),
                                GITHUB_WORKSPACE=str(self.root / "workspace"),
                                GITHUB_ENV=str(self.root / "environment"),
                                GITHUB_OUTPUT=str(self.root / "outputs"), PATH=str(self.root))

    def run_script(self, phase):
        return subprocess.run([self.shell, "-NoProfile", "-File", str(self.script), "-Phase", phase],
                              env=self.environment, capture_output=True, text=True, timeout=30)

    @unittest.skipIf(os.name == "nt", "portable executable fixture uses a Unix symlink")
    def test_existing_interpreter_needs_no_download_or_path_mutation(self):
        (self.root / "python.exe").symlink_to(sys.executable)
        result = self.run_script("probe")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("needed=false", (self.root / "outputs").read_text())
        self.assertIn("PLUMB_WORKFLOW_PYTHON=", (self.root / "environment").read_text())
        self.assertFalse(any(self.root.glob("*.zip")))
        self.assertEqual(self.environment["PATH"], str(self.root))

    def test_missing_interpreter_selects_fixed_archive_without_downloading(self):
        result = self.run_script("probe")
        self.assertEqual(result.returncode, 0, result.stderr)
        output = (self.root / "outputs").read_text()
        self.assertIn("needed=true", output)
        self.assertIn("windows-x64-python-3.13.15-v1", output)
        self.assertFalse((self.root / "environment").exists())
        self.assertFalse(any(self.root.rglob("*.zip")))

    def test_invalid_cached_archive_refuses_before_execution(self):
        cache = self.root / "windows-x64-python-3.13.15-v1"
        cache.mkdir()
        archive = cache / "python.zip"
        archive.write_bytes(b"corrupt")
        result = self.run_script("install")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("checksum mismatch", result.stderr)
        self.assertEqual(archive.read_bytes(), b"corrupt")
        self.assertFalse((self.root / "environment").exists())

    def test_archive_cache_identity_does_not_include_the_job_root(self):
        outputs = []
        for name in ("first-job", "second-job"):
            seat = self.root / name
            temporary = seat / "tmp"
            temporary.mkdir(parents=True)
            self.environment.update(RUNNER_TEMP=str(temporary),
                                    GITHUB_WORKSPACE=str(seat / "workspace"),
                                    GITHUB_OUTPUT=str(temporary / "outputs"))
            result = self.run_script("probe")
            self.assertEqual(result.returncode, 0, result.stderr)
            outputs.append((temporary / "outputs").read_text())
        self.assertEqual(outputs[0], outputs[1])
        self.assertIn("archive=../tmp/windows-x64-python-3.13.15-v1/python.zip", outputs[0])
        self.assertNotIn(str(self.root), outputs[0])
