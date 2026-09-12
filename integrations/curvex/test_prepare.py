"""Filesystem isolation regressions for the Curvex migration entry points.

Run: python3 -m unittest discover -s integrations/curvex -p 'test_prepare.py'
"""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


HERE = Path(__file__).resolve().parent


class MigrationIsolationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="curvex-isolation-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.source = self.root / "original"
        (self.source / "src/ui").mkdir(parents=True)
        (self.source / "Cargo.toml").write_text(
            '[package]\nname = "curvex"\nversion = "0.0.0"\n'
            '[dependencies]\nkurbo = "0.11"\n'
        )
        (self.source / "src/lib.rs").write_text("pub use kurbo::Point;\n")
        (self.source / "src/ui/untouched.txt").write_bytes(b"original sentinel\x00\xff")
        self.before = self.snapshot()

    def snapshot(self):
        return {
            str(path.relative_to(self.source)): path.read_bytes() if path.is_file() else None
            for path in sorted(self.source.rglob("*"))
        }

    def run_entry(self, script, *arguments):
        return subprocess.run(
            [sys.executable, str(HERE / script), str(self.source), *map(str, arguments)],
            capture_output=True, text=True, timeout=5,
        )

    def assert_rejected(self, script, destination, *options, label="destination"):
        result = self.run_entry(script, destination, *options)
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn(f"{label} must be outside the original source checkout", result.stderr)
        self.assertEqual(self.snapshot(), self.before, "rejection must leave all original files and directories unchanged")
        if destination != self.source:
            self.assertFalse(destination.exists(), "a rejected destination must never be created")

    def test_prepare_rejects_original_and_nested_destination_before_copy(self):
        for destination in (self.source, self.source / "migration", self.source / "src/ui/migration"):
            with self.subTest(destination=destination):
                self.assert_rejected("prepare.py", destination)

    def test_prepare_resolves_symlink_aliases_before_checking_destination(self):
        alias = self.root / "source-alias"
        alias.symlink_to(self.source, target_is_directory=True)
        destination = alias / "migration"
        self.assert_rejected("prepare.py", destination)

    def test_prepare_allows_separate_copy_and_preserves_original_bytes(self):
        destination = self.root / "migrated"
        result = self.run_entry("prepare.py", destination)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.snapshot(), self.before)
        self.assertEqual((destination / "src/lib.rs").read_text(), "pub use osv_geometry::curve::Point;\n")
        self.assertTrue((destination / "src/ui/osv_gradient_sampling.rs").is_file())
        self.assertTrue((destination / "osv-migration.patch").is_file())

    def test_qualification_rejects_every_original_write_root_before_staging(self):
        for label in ("destination", "target", "baseline"):
            for protected in (self.source, self.source / "nested"):
                with self.subTest(label=label, protected=protected):
                    destination = protected if label == "destination" else self.root / "qualification-copy"
                    target = protected if label == "target" else self.root / "cargo-target"
                    options = ["--target", target]
                    if label == "baseline":
                        options += ["--baseline", protected]
                    self.assert_rejected("qualify.py", destination, *options, label=label)
                    self.assertFalse((self.root / "cargo-target").exists())

    def test_qualification_rejects_target_and_baseline_symlink_aliases(self):
        alias = self.root / "source-alias"
        alias.symlink_to(self.source, target_is_directory=True)
        for label in ("target", "baseline"):
            with self.subTest(label=label):
                options = ["--target", alias / "target" if label == "target" else self.root / "target"]
                if label == "baseline":
                    options += ["--baseline", alias]
                self.assert_rejected("qualify.py", self.root / "copy", *options, label=label)

    def test_qualification_rejects_baseline_examples_symlink_into_original(self):
        baseline = self.root / "separate-baseline"
        baseline.mkdir()
        (baseline / "examples").symlink_to(self.source / "src/ui", target_is_directory=True)
        self.assert_rejected(
            "qualify.py", self.root / "copy", "--target", self.root / "target",
            "--baseline", baseline, label="baseline example",
        )


if __name__ == "__main__":
    unittest.main()
