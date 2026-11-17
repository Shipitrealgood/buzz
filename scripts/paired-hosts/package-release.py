#!/usr/bin/env python3
"""Build a private, checksummed host release from a clean pinned Linux checkout."""

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path


def output(*command):
    return subprocess.check_output(command, text=True).strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("commit", help="full commit ID of this source checkout")
    parser.add_argument("package", type=Path, help="new package directory")
    args = parser.parse_args()
    if sys.platform != "linux" or Path(output("git", "rev-parse", "--show-toplevel")) != Path.cwd():
        parser.error("run at the root of a Linux source checkout")
    if args.package.exists() or args.package.is_symlink():
        parser.error("package destination already exists")

    def check_pin():
        if output("git", "rev-parse", "HEAD") != args.commit or output("git", "status", "--porcelain"):
            parser.error("the source checkout must be clean at the requested commit")

    # Build natively so the resulting tools can run on this Linux host.
    if os.environ.get("CARGO_BUILD_TARGET"):
        parser.error("unset CARGO_BUILD_TARGET for a native host release")
    check_pin()
    binaries = ("buzz", "buzz-acp", "buzz-host-runtime", "buzz-hostd")
    build = [
        "cargo", "build", "--offline", "--locked", "--release", "--jobs", "2",
        "-p", "buzz-hostd", "--bin", "buzz-hostd", "--bin", "buzz-host-runtime",
        "-p", "buzz-acp", "--bin", "buzz-acp", "-p", "buzz-cli", "--bin", "buzz",
        "--message-format=json-render-diagnostics",
    ]
    artifacts = {}
    for line in output(*build).splitlines():
        record = json.loads(line)
        if record.get("executable") and record["target"]["name"] in binaries:
            artifacts[record["target"]["name"]] = Path(record["executable"])
    if artifacts.keys() != set(binaries):
        sys.exit("Cargo did not produce the four requested host binaries")
    check_pin()

    os.umask(0o077)
    args.package.mkdir()
    for name, path in artifacts.items():
        shutil.copyfile(path, args.package / name)
        (args.package / name).chmod(0o700)
    source = {
        "source_commit": args.commit,
        "recipe_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "cargo_lock_sha256": hashlib.sha256(Path("Cargo.lock").read_bytes()).hexdigest(),
        "rustc": output("rustc", "-vV"),
        "cargo": output("cargo", "--version"),
        "build": build,
    }
    (args.package / "SOURCE.json").write_text(json.dumps(source, indent=2) + "\n")
    with (args.package / "MANIFEST.sha256").open("x") as manifest:
        subprocess.run(["sha256sum", *binaries, "SOURCE.json"], cwd=args.package, stdout=manifest, check=True)
    subprocess.run(["sha256sum", "--check", "MANIFEST.sha256"], cwd=args.package, check=True)


if __name__ == "__main__":
    main()
