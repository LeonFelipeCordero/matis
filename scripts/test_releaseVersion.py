import tempfile
import tomllib
import unittest
from pathlib import Path

from releaseVersion import next_version, semver, stamp_version


class ReleaseVersionTests(unittest.TestCase):
    def test_first_release_uses_manifest_version(self):
        self.assertEqual(next_version("0.1.0", [], []), "0.1.0")

    def test_push_increments_highest_stable_patch(self):
        self.assertEqual(next_version("0.1.0", ["v0.1.9", "v0.1.10", "v2.0.0-rc.1", "other"], []), "0.1.11")

    def test_explicit_minor_or_major_baseline_wins(self):
        for baseline in ["0.2.0", "1.0.0"]:
            with self.subTest(baseline=baseline):
                self.assertEqual(next_version(baseline, ["v0.1.9"], []), baseline)

    def test_rerun_reuses_tag_on_same_commit(self):
        self.assertEqual(next_version("0.1.0", ["v0.1.2", "v0.1.3"], ["v0.1.2"]), "0.1.2")

    def test_invalid_baselines_fail(self):
        for version in ["v1.0.0", "1.0", "01.0.0", "1.0.0-beta"]:
            with self.subTest(version=version), self.assertRaises(ValueError):
                semver(version)

    def test_stamp_updates_only_matis_package(self):
        with tempfile.TemporaryDirectory() as directory:
            manifest = Path(directory) / "Cargo.toml"
            lockfile = Path(directory) / "Cargo.lock"
            manifest.write_text('[package]\nname = "matis-rust"\nversion = "0.1.0"\n\n[dependencies]\nother = { version = "0.1.0" }\n')
            lockfile.write_text('version = 4\n\n[[package]]\nname = "matis-rust"\nversion = "0.1.0"\n\n[[package]]\nname = "other"\nversion = "0.1.0"\n')
            stamp_version(manifest, lockfile, "0.1.1")
            self.assertEqual(tomllib.loads(manifest.read_text())["package"]["version"], "0.1.1")
            self.assertEqual(tomllib.loads(manifest.read_text())["dependencies"]["other"]["version"], "0.1.0")
            self.assertEqual([p["version"] for p in tomllib.loads(lockfile.read_text())["package"]], ["0.1.1", "0.1.0"])

    def test_missing_lock_package_does_not_change_manifest(self):
        with tempfile.TemporaryDirectory() as directory:
            manifest = Path(directory) / "Cargo.toml"
            lockfile = Path(directory) / "Cargo.lock"
            original = '[package]\nname = "matis-rust"\nversion = "0.1.0"\n'
            manifest.write_text(original)
            lockfile.write_text("version = 4\n")
            with self.assertRaises(ValueError):
                stamp_version(manifest, lockfile, "0.1.1")
            self.assertEqual(manifest.read_text(), original)


if __name__ == "__main__":
    unittest.main()
