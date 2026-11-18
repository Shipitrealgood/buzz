#!/usr/bin/env python3
"""Select one packaged release for hosted auto-discovery and agent shells."""

import argparse
import hashlib
import json
import os
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("package", type=Path, help="installed or staged release directory")
    parser.add_argument("catalog", type=Path, help="existing local auto-discovery deployment.json")
    parser.add_argument("candidate", type=Path, help="new file for review")
    args = parser.parse_args()
    package = args.package.resolve(strict=True)
    if args.candidate.exists() or args.candidate.is_symlink():
        parser.error("candidate destination already exists")
    if ":" in str(package) or not str(package).isprintable():
        parser.error("package cannot be represented as a PATH entry")

    manifest = (package / "MANIFEST.sha256").read_text().splitlines()
    for name in ("buzz", "buzz-acp"):
        binary = package / name
        if binary.is_symlink() or not binary.is_file() or not os.access(binary, os.X_OK):
            parser.error(f"missing regular executable: {binary}")
        with binary.open("rb") as stream:
            digest = hashlib.file_digest(stream, "sha256").hexdigest()
        entries = [line for line in manifest if line.partition("  ")[2] == name]
        if entries != [f"{digest}  {name}"]:
            parser.error(f"release manifest does not match {name}")

    catalog = json.loads(args.catalog.read_text())
    local = catalog["local"]
    if catalog["runtimes"] or not Path(local["buzz_acp"]).is_absolute() or not local["tool_path"]:
        parser.error("expected an auto-discovery catalog with local.buzz_acp and local.tool_path")
    previous = Path(local["buzz_acp"]).parent
    paths = local["tool_path"].split(":")
    if paths[0] == str(previous) and (previous / "MANIFEST.sha256").is_file():
        paths.pop(0)
    local["buzz_acp"] = str(package / "buzz-acp")
    local["tool_path"] = ":".join([str(package), *(p for p in paths if p != str(package))])

    os.umask(0o077)
    with args.candidate.open("x") as candidate:
        json.dump(catalog, candidate, indent=2)
        candidate.write("\n")


if __name__ == "__main__":
    main()
