#!/usr/bin/env python3
"""Exercise two host release selections against a conflicting account CLI."""

import json
import subprocess
import tempfile
import unittest
from pathlib import Path


SELECT = Path(__file__).with_name("stage-release-catalog.py")


class ReleaseCatalogTest(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        self.account = self.root / "account"
        self.account.mkdir()
        (self.account / "buzz").write_text("#!/bin/sh\necho stale\n")
        (self.account / "buzz").chmod(0o700)
        self.catalog = self.root / "deployment.json"
        self.original = {
            "runtimes": {}, "capacity": 1, "label": "test",
            "local": {"buzz_acp": "/legacy/buzz-acp", "tool_path": str(self.account),
                      "launcher": "/existing/buzz-host-runtime"},
        }
        self.catalog.write_text(json.dumps(self.original))

    def package(self, release):
        path = self.root / release
        path.mkdir()
        for name in ("buzz", "buzz-acp"):
            binary = path / name
            binary.write_text(f"#!/bin/sh\necho {release}/{name}\n")
            binary.chmod(0o700)
        with (path / "MANIFEST.sha256").open("w") as manifest:
            subprocess.run(["sha256sum", "buzz", "buzz-acp"], cwd=path, stdout=manifest, check=True)
        return path

    def test_upgrade_selects_both_from_one_release(self):
        old = self.catalog
        for release in ("first", "second"):
            package = self.package(release)
            candidate = self.root / f"{release}.json"
            subprocess.run([SELECT, package, old, candidate], check=True)
            saved = json.loads(candidate.read_text())
            local = saved["local"]
            self.assertEqual(candidate.stat().st_mode & 0o777, 0o600)
            self.assertEqual(local["tool_path"].split(":"), [str(package), str(self.account)])
            env = {"PATH": local["tool_path"]}
            for command in ("buzz", local["buzz_acp"]):
                actual = subprocess.check_output([command], env=env, text=True).strip()
                self.assertEqual(actual, f"{release}/{Path(command).name}")
            self.assertEqual(local["launcher"], self.original["local"]["launcher"])
            old = candidate
        self.assertEqual(json.loads(self.catalog.read_text()), self.original)

    def test_corrupted_or_missing_member_refuses_candidate(self):
        for name in ("buzz", "buzz-acp"):
            with self.subTest(name=name):
                package = self.package(name)
                binary = package / name
                if name == "buzz":
                    binary.write_text("changed")
                else:
                    binary.unlink()
                candidate = self.root / f"{name}.json"
                result = subprocess.run([SELECT, package, self.catalog, candidate], capture_output=True)
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse(candidate.exists())

    def test_explicit_runtime_and_existing_destination_are_preserved(self):
        package = self.package("release")
        candidate = self.root / "candidate"
        candidate.write_text("retain")
        result = subprocess.run([SELECT, package, self.catalog, candidate], capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(candidate.read_text(), "retain")
        self.original["runtimes"]["custom"] = {}
        self.catalog.write_text(json.dumps(self.original))
        fresh = self.root / "fresh.json"
        result = subprocess.run([SELECT, package, self.catalog, fresh], capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(fresh.exists())


if __name__ == "__main__":
    unittest.main()
