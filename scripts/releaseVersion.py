"""Select a release version and stamp the build without changing main."""

import re
import sys
import subprocess
import tomllib
from pathlib import Path


def semver(value):
    match = re.fullmatch(r"(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)", value)
    if not match:
        raise ValueError(f"Expected a stable major.minor.patch version: {value}")
    return tuple(map(int, match.groups()))


def next_version(base, tags, current_tags):
    baseline = semver(base)
    versions = {tag: semver(tag[1:]) for tag in tags if re.fullmatch(r"v(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)", tag)}
    current = [versions[tag] for tag in current_tags if tag in versions]
    if current:
        return ".".join(map(str, max(current)))
    if not versions or baseline > max(versions.values()):
        return base
    major, minor, patch = max(versions.values())
    return f"{major}.{minor}.{patch + 1}"


def stamp_version(manifest, lockfile, version):
    semver(version)
    source = manifest.read_text()
    package = tomllib.loads(source)["package"]
    lock = lockfile.read_text()
    old_package = f'name = "{package["name"]}"\nversion = "{package["version"]}"'
    if lock.count(old_package) != 1:
        raise ValueError("Cargo.lock must contain exactly one matching Matis package")
    updated = source.replace(f'version = "{package["version"]}"', f'version = "{version}"', 1)
    if tomllib.loads(updated)["package"]["version"] != version:
        raise ValueError("Could not update the package version")
    manifest.write_text(updated)
    lockfile.write_text(lock.replace(old_package, f'name = "{package["name"]}"\nversion = "{version}"', 1))


def main():
    manifest = Path("matis-rust/Cargo.toml")
    if len(sys.argv) == 3 and sys.argv[1] == "stamp":
        stamp_version(manifest, Path("matis-rust/Cargo.lock"), sys.argv[2])
        return
    base = tomllib.loads(manifest.read_text())["package"]["version"]
    tags = subprocess.check_output(["git", "tag", "--list"], text=True).splitlines()
    current = subprocess.check_output(["git", "tag", "--points-at", "HEAD"], text=True).splitlines()
    print(next_version(base, tags, current))


if __name__ == "__main__":
    main()
